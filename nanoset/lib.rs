#![feature(doc_cfg, external_doc)]
#![doc(include = "../README.md")]

extern crate pyo3;
extern crate pyo3_built;

mod built;

use pyo3::class::basic::CompareOp;
use pyo3::class::PyGCProtocol;
use pyo3::class::PyIterProtocol;
use pyo3::class::PyNumberProtocol;
use pyo3::class::PyObjectProtocol;
use pyo3::class::PySequenceProtocol;
use pyo3::exceptions::KeyError;
use pyo3::gc::PyTraverseError;
use pyo3::gc::PyVisit;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3::types::PyDict;
use pyo3::types::PyFrozenSet;
use pyo3::types::PyIterator;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use pyo3::AsPyPointer;
use pyo3::PyNativeType;

// --- Common implementation -------------------------------------------------

macro_rules! common_impl {
    ($cls:ty) => {
        impl $cls {
            pub fn new() -> Self {
                Self::default()
            }

            pub fn from_set(obj: PyObject) -> Self {
                Self { inner: Some(obj) }
            }

            pub fn try_from_any(py: Python, any: &PyAny) -> PyResult<Self> {
                Self::try_from_obj(py, any.to_object(py))
            }

            pub fn try_from_obj(py: Python, obj: PyObject) -> PyResult<Self> {
                if let Ok(s) = obj.cast_as::<PySet>(py) {
                    if s.is_empty() {
                        Ok(Self::new())
                    } else {
                        s.to_object(py).call_method0(py, "copy").map(Self::from_set)
                    }
                } else if let Ok(d) = obj.cast_as::<PyDict>(py) {
                    if d.is_empty() {
                        Ok(Self::new())
                    } else {
                        unsafe {
                            let set = pyo3::ffi::PySet_New(d.as_ptr());
                            Ok(Self::from_set(PyObject::from_owned_ptr(py, set)))
                        }
                    }
                } else {
                    let iterator = PyIterator::from_object(py, &obj)?;
                    Self::try_from_iterator(py, iterator)
                }
            }

            pub fn try_from_iterator(py: Python, it: PyIterator) -> PyResult<Self> {
                let items: PyResult<Vec<&PyAny>> = it.collect();
                let res = items?;

                if res.is_empty() {
                    Ok(Self::default())
                } else {
                    let set = PySet::new(py, res.as_slice())?;
                    Ok(Self::from_set(set.to_object(py)))
                }
            }
        }

        impl FromPy<PySet> for $cls {
            fn from_py(set: PySet, py: Python) -> Self {
                Self::from_set(set.to_object(py))
            }
        }

        impl Drop for $cls {
            fn drop(&mut self) {
                if let Some(set) = self.inner.take() {
                    Python::acquire_gil().python().release(set)
                }
            }
        }

        #[pymethods]
        impl $cls {
            #[new]
            fn __new__(iterable: Option<&PyAny>) -> PyResult<Self> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                let cell = PyCell::new(py,  Self::new())?;
                Self::__init__(cell, iterable)?;
                Ok(cell.replace(Self::new()))
            }

            fn __init__(slf: &PyCell<Self>, iterable: Option<&PyAny>) -> PyResult<()> {
                if let Some(it) = iterable {
                    let gil = Python::acquire_gil();
                    let py = gil.python();
                    if let Ok(set) = it.extract::<PyRef<Self>>() {
                        slf.replace(set.copy()?);
                    } else {
                        slf.replace(Self::try_from_any(py, it)?);
                    }
                } else {
                    slf.replace(Self::new());
                }
                Ok(())
            }

            fn __getstate__(&self) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match self.inner {
                    None => Ok(py.None()),
                    Some(ref set) => Ok(set.clone_ref(py)),
                }
            }

            fn __setstate__(slf: &PyCell<Self>, state: PyObject) -> PyResult<()> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                // check that we got either `None`, or a set
                let inner = if state.is_none() {
                    None
                } else if state.cast_as::<PySet>(py)?.is_empty() {
                    None
                } else {
                    Some(state)
                };

                slf.borrow_mut().inner = inner;
                Ok(())
            }

            fn __reduce__(&self) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                let ty = <$cls as pyo3::type_object::PyTypeObject>::type_object();

                match self.inner {
                    None => Ok((ty, PyTuple::empty(py)).to_object(py)),
                    Some(ref set) => Ok((ty, (set.call_method0(py, "copy")?,)).to_object(py)),
                }
            }

            fn add(slf: &PyCell<Self>, item: &PyAny) -> PyResult<()> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                let inner = match slf.borrow().inner.as_ref() {
                    None => PySet::empty(py)?.to_object(py),
                    Some(obj) => obj.clone_ref(py),
                };

                inner.cast_as::<PySet>(py)?.add(item)?;
                slf.borrow_mut().inner = Some(inner);
                Ok(())
            }

            fn clear(slf: &PyCell<Self>) -> PyResult<()> {
                slf.borrow_mut().inner = None;
                Ok(())
            }

            fn copy(&self) -> PyResult<Self> {
                match self.inner {
                    None => Ok(Self::new()),
                    Some(ref inner) => {
                        let gil = Python::acquire_gil();
                        let py = gil.python();
                        inner.call_method0(py, "copy").map(Self::from_set)
                    }
                }
            }

            #[args(others = "*")]
            fn difference(slf: &PyCell<Self>, others: &PyTuple) -> PyResult<Self> {
                // check if we got an argument, otherwise just copy the current
                // set as the result
                if others.is_empty() {
                    return slf.borrow().copy();
                }

                // get the inner set object or return an empty one
                let py = others.py();
                let inner = match slf.borrow_mut().inner.as_ref() {
                    None => PySet::empty(py)?.to_object(py),
                    Some(obj) => obj.clone_ref(py),
                };

                // create the union and wrap it in a new NanoSet
                inner.call_method1(py, "difference", others)
                    .and_then(|obj| Self::try_from_obj(py, obj))
            }

            #[args(others = "*")]
            fn difference_update(slf: &PyCell<Self>, others: &PyTuple) -> PyResult<()> {
                // bail out early if we are not given any argument
                if others.is_empty() {
                    return Ok(());
                }

                // get the inner set object if the set is not empty
                let py = others.py();
                let inner = match slf.borrow_mut().inner.as_ref() {
                    None => PySet::empty(py)?.to_object(py),
                    Some(obj) => obj.clone_ref(py),
                };

                // update with the given arguments
                inner.call_method1(py, "difference_update", others)?;
                if inner.cast_as::<PySet>(py)?.is_empty() {
                    slf.borrow_mut().inner = None;
                }

                Ok(())
            }

            fn discard(slf: &PyCell<Self>, elem: &PyAny) -> PyResult<()> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                let inner = match slf.borrow().inner {
                    None => return Ok(()),
                    Some(ref obj) => obj.clone_ref(py),
                };

                inner.call_method1(py, "discard", (elem,))?;
                if inner.cast_as::<PySet>(py)?.is_empty() {
                    slf.borrow_mut().inner = None;
                }
                Ok(())
            }

            #[args(others = "*")]
            fn intersection(slf: &PyCell<Self>, others: &PyTuple) -> PyResult<Self> {
                // check if we got an argument, otherwise just copy the current
                // set as the result
                if others.is_empty() {
                    return slf.borrow().copy();
                }

                // get the inner set object or return an empty one since
                // intersection with an empty set is always empty
                let py = others.py();
                let inner = match slf.borrow().inner {
                    Some(ref obj) => obj.clone_ref(py),
                    None => PySet::empty(py)?.to_object(py),
                };

                // create the union and wrap it in a new NanoSet
                inner.call_method1(py, "intersection", others)
                    .and_then(|obj| Self::try_from_obj(py, obj))
            }

            #[args(others = "*")]
            fn intersection_update(slf: &PyCell<Self>, others: &PyTuple) -> PyResult<()> {
                // bail out early if we are not given any argument
                if others.is_empty() {
                    return Ok(());
                }

                // get the inner set object if the set is not empty
                let py = others.py();
                let inner = match slf.borrow_mut().inner.as_ref() {
                    None => PySet::empty(py)?.to_object(py),
                    Some(obj) => obj.clone_ref(py),
                };

                // update with the given arguments
                inner.call_method1(py, "intersection_update", others)?;
                if inner.cast_as::<PySet>(py)?.is_empty() {
                    slf.borrow_mut().inner = None;
                }

                Ok(())
            }

            fn isdisjoint(slf: &PyCell<Self>, other: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                let inner = match slf.borrow().inner.as_ref() {
                    None => PySet::empty(py)?.to_object(py),
                    Some(obj) => obj.clone_ref(py),
                };

                inner.call_method1(py, "isdisjoint", (other,))
            }

            fn issubset(slf: &PyCell<Self>, other: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                let inner = match slf.borrow().inner.as_ref() {
                    None => PySet::empty(py)?.to_object(py),
                    Some(obj) => obj.clone_ref(py),
                };

                inner.call_method1(py, "issubset", (other,))
            }

            fn issuperset(slf: &PyCell<Self>, other: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                let inner = match slf.borrow().inner.as_ref() {
                    None => PySet::empty(py)?.to_object(py),
                    Some(obj) => obj.clone_ref(py),
                };

                inner.call_method1(py, "issuperset", (other,))
            }

            fn pop(slf: &PyCell<Self>) -> PyResult<PyObject> {
                // get the inner set if it is not empty
                let gil = Python::acquire_gil();
                let py = gil.python();
                let inner = match slf.borrow().inner {
                    None => return KeyError::into("pop from an empty set"),
                    Some(ref inner) => inner.clone_ref(py),
                };

                // pop from the set (which is not empty)
                let set = inner.cast_as::<PySet>(py)
                    .expect("inner set is always a `PySet`");
                let item = set.pop().expect("inner set is never empty");

                // take care to clear the inner set if we exhausted it
                if set.is_empty() {
                    slf.borrow_mut().inner = None;
                }

                Ok(item)
            }

            fn remove(slf: &PyCell<Self>, item: &PyAny) -> PyResult<()> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                let inner = match slf.borrow().inner {
                    None => return KeyError::into(item.to_object(py)),
                    Some(ref obj) => obj.clone_ref(py),
                };

                // `set2.remove(set1)` actually does for
                // `set2.remove(frozenset(set1))`, so we have to check if
                // `set1` is `NanoSet` to reproduce that behaviour.
                if let Ok(ref other) = item.extract::<PyRef<$cls>>() {
                    if let Some(ref obj) = other.inner {
                        inner.call_method1(py, "remove", (obj.clone_ref(py),))?
                    } else {
                        inner.call_method1(py, "remove", (PyFrozenSet::empty(py)?,))?
                    };
                } else {
                    inner.call_method1(py, "remove", (item,))?;
                }

                // after removing the item we check if the set is empty
                // to maintain the invariant
                if inner.cast_as::<PySet>(py).unwrap().is_empty() {
                    slf.borrow_mut().inner = None
                }

                Ok(())
            }

            fn symmetric_difference(slf: &PyCell<Self>, other: &PyAny) -> PyResult<Self> {
                // get the inner set or create a new one
                let gil = Python::acquire_gil();
                let py = gil.python();
                let inner = match slf.borrow().inner.as_ref() {
                    None => PySet::empty(py)?.to_object(py),
                    Some(s) => s.clone_ref(py),
                };

                // compute the symmetric difference
                inner.call_method1(py, "symmetric_difference", (other,))
                    .and_then(|obj| Self::try_from_obj(py, obj))
            }

            fn symmetric_difference_update(slf: &PyCell<Self>, other: &PyAny) -> PyResult<()> {
                // get the inner set object or create a new one
                let gil = Python::acquire_gil();
                let py = gil.python();
                let inner = match slf.borrow_mut().inner.as_ref() {
                    None => PySet::empty(py)?.to_object(py),
                    Some(obj) => obj.clone_ref(py),
                };

                // update with the given arguments and update the wrapped object
                inner.call_method1(py, "symmetric_difference_update", (other,))?;
                if !inner.cast_as::<PySet>(py)?.is_empty() {
                    slf.borrow_mut().inner = Some(inner);
                }

                Ok(())
            }

            #[args(others = "*")]
            fn union(slf: &PyCell<Self>, others: &PyTuple) -> PyResult<Self> {
                // check if we got an argument, otherwise just copy the current
                // set as the result
                if others.is_empty() {
                    return slf.borrow().copy();
                }

                // get the inner set object or create a new one
                let py = others.py();
                let inner = match slf.borrow_mut().inner.as_ref() {
                    None => PySet::empty(py)?.to_object(py),
                    Some(obj) => obj.clone_ref(py),
                };

                // create the union and wrap it in a new NanoSet
                inner.call_method1(py, "union", others)
                    .and_then(|obj| Self::try_from_obj(py, obj))
            }

            #[args(others = "*")]
            fn update(slf: &PyCell<Self>, others: &PyTuple) -> PyResult<()> {
                // only attempt to borrow self if we are actually given some
                // arguments to process
                if !others.is_empty() {
                    // get the inner set object or create a new one
                    let py = others.py();
                    let inner = match slf.borrow_mut().inner.take() {
                        None => PySet::empty(py)?.to_object(py),
                        Some(obj) => obj.clone_ref(py),
                    };

                    // update with the given arguments and update the wrapped
                    // or set it to the new set only if it is not empty
                    inner.call_method1(py, "update", others)?;
                    if !inner.cast_as::<PySet>(py)?.is_empty() {
                        slf.borrow_mut().inner = Some(inner);
                    }
                }

                Ok(())
            }
        }

        #[pyproto]
        impl PyIterProtocol for $cls {
            fn __iter__(slf: PyRefMut<Self>) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match slf.inner.as_ref() {
                    None => PyTuple::empty(py)
                        .to_object(py)
                        .call_method0(py, "__iter__"),
                    Some(inner) => inner
                        .call_method0(py, "__iter__"),
                }
            }
        }

        #[pyproto]
        impl PyNumberProtocol for $cls {
            fn __and__(lhs: &PyCell<Self>, rhs: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                if rhs.cast_as::<PySet>().is_err()
                    && rhs.cast_as::<PyFrozenSet>().is_err()
                    && rhs.extract::<PyRef<$cls>>().is_err()
                {
                    return Ok(py.NotImplemented());
                }

                let args: Py<PyTuple> = (rhs,).into_py(gil.python());
                Self::intersection(lhs, &args.as_ref(py))
                    .and_then(|s| Py::new(py, s))
                    .map(PyObject::from)
            }

            fn __sub__(lhs: &PyCell<Self>, rhs: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                if rhs.cast_as::<PySet>().is_err()
                    && rhs.cast_as::<PyFrozenSet>().is_err()
                    && rhs.extract::<PyRef<$cls>>().is_err()
                {
                    return Ok(py.NotImplemented());
                }

                let args: Py<PyTuple> = (rhs,).into_py(gil.python());
                Self::difference(lhs, &args.as_ref(py))
                    .and_then(|s| Py::new(py, s))
                    .map(PyObject::from)
            }

            fn __or__(lhs: &PyCell<Self>, rhs: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                if rhs.cast_as::<PySet>().is_err()
                    && rhs.cast_as::<PyFrozenSet>().is_err()
                    && rhs.extract::<PyRef<$cls>>().is_err()
                {
                    return Ok(py.NotImplemented());
                }

                let args: Py<PyTuple> = (rhs,).into_py(gil.python());
                Self::union(lhs, &args.as_ref(py))
                    .and_then(|s| Py::new(py, s))
                    .map(PyObject::from)
            }

            fn __xor__(lhs: &PyCell<Self>, rhs: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                if rhs.cast_as::<PySet>().is_err()
                    && rhs.cast_as::<PyFrozenSet>().is_err()
                    && rhs.extract::<PyRef<$cls>>().is_err()
                {
                    return Ok(py.NotImplemented());
                }

                Self::symmetric_difference(lhs, rhs)
                    .and_then(|s| Py::new(py, s))
                    .map(PyObject::from)
            }
        }

        #[pyproto]
        impl PyObjectProtocol for $cls {
            fn __repr__(&self) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match self.inner {
                    None => {
                        // let s = concat!(stringify!($cls), "()");
                        // Ok(s.to_object(py))
                        Ok("set()".to_object(py))
                    }
                    Some(ref inner) => {
                        // let s = PyString::new(py, concat!(stringify!($cls), "({})"));
                        // s.to_object(py).call_method1(py, "format", (inner,))
                        inner.call_method0(py, "__repr__")
                    }
                }
            }

            fn __bool__(&self) -> PyResult<bool> {
                Ok(self.inner.is_some())
            }

            fn __richcmp__(&self, obj: &PyAny, op: CompareOp) -> PyResult<PyObject> {
                use self::CompareOp::*;

                let gil = Python::acquire_gil();
                let py = gil.python();

                if let Ok(other) = obj.extract::<PyRef<Self>>() {
                    match (&self.inner, &other.inner) {
                        (None, None) => match op {
                            Eq | Le | Ge => Ok(true.to_object(py)),
                            Ne | Lt | Gt => Ok(false.to_object(py)),
                        },
                        (None, Some(_)) => match op {
                            Eq | Gt | Ge => Ok(false.to_object(py)),
                            Ne | Lt | Le => Ok(true.to_object(py)),
                        },
                        (Some(_), None) => match op {
                            Eq | Lt | Le => Ok(false.to_object(py)),
                            Ne | Gt | Ge => Ok(true.to_object(py)),
                        },
                        (Some(l), Some(r)) => match op {
                            Eq => l.call_method1(py, "__eq__", (r,)),
                            Ne => l.call_method1(py, "__ne__", (r,)),
                            Lt => l.call_method1(py, "__lt__", (r,)),
                            Le => l.call_method1(py, "__le__", (r,)),
                            Gt => l.call_method1(py, "__gt__", (r,)),
                            Ge => l.call_method1(py, "__ge__", (r,)),
                        },
                    }
                } else if let Ok(other) = obj.cast_as::<PySet>() {
                    match (&self.inner, other.is_empty()) {
                        (None, true) => match op {
                            Eq | Le | Ge => Ok(true.to_object(py)),
                            Ne | Lt | Gt => Ok(false.to_object(py)),
                        },
                        (None, false) => match op {
                            Eq | Gt | Ge => Ok(false.to_object(py)),
                            Ne | Lt | Le => Ok(true.to_object(py)),
                        },
                        (Some(l), _) => match op {
                            Eq => l.call_method1(py, "__eq__", (obj,)),
                            Ne => l.call_method1(py, "__ne__", (obj,)),
                            Lt => l.call_method1(py, "__lt__", (obj,)),
                            Le => l.call_method1(py, "__le__", (obj,)),
                            Gt => l.call_method1(py, "__gt__", (obj,)),
                            Ge => l.call_method1(py, "__ge__", (obj,)),
                        },
                    }
                } else if let Ok(other) = obj.cast_as::<PyFrozenSet>() {
                    match (&self.inner, other.is_empty()) {
                        (None, true) => match op {
                            Eq | Le | Ge => Ok(true.to_object(py)),
                            Ne | Lt | Gt => Ok(false.to_object(py)),
                        },
                        (None, false) => match op {
                            Eq | Gt | Ge => Ok(false.to_object(py)),
                            Ne | Lt | Le => Ok(true.to_object(py)),
                        },
                        (Some(l), _) => match op {
                            Eq => l.call_method1(py, "__eq__", (obj,)),
                            Ne => l.call_method1(py, "__ne__", (obj,)),
                            Lt => l.call_method1(py, "__lt__", (obj,)),
                            Le => l.call_method1(py, "__le__", (obj,)),
                            Gt => l.call_method1(py, "__gt__", (obj,)),
                            Ge => l.call_method1(py, "__ge__", (obj,)),
                        },
                    }
                } else {
                    match op {
                        CompareOp::Eq => Ok(false.to_object(py)),
                        CompareOp::Ne => Ok(true.to_object(py)),
                        _ => Ok(py.NotImplemented()),
                    }
                }
            }
        }

        #[pyproto]
        impl PySequenceProtocol for $cls {
            fn __len__(&self) -> PyResult<usize> {
                match self.inner {
                    None => Ok(0usize),
                    Some(ref inner) => {
                        let gil = Python::acquire_gil();
                        Ok(inner.cast_as::<PySet>(gil.python())?.len())
                    }
                }
            }

            fn __contains__(&self, item: &PyAny) -> PyResult<bool> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                if let Some(ref obj) = self.inner {
                    // `set1 in set2` actually checks for
                    // `frozenset(set1) in set2`, so we have to check if
                    // `set1` is `NanoSet` to reproduce that behaviour.
                    if let Ok(ref other) = item.extract::<PyRef<$cls>>() {
                        let set = obj.cast_as::<PySet>(py).unwrap();
                        match other.inner {
                            Some(ref obj) => set.contains(obj),
                            None => set.contains(PyFrozenSet::empty(py)?),
                        }
                    } else {
                        obj.call_method1(py, "__contains__", (item,))
                            .and_then(|val| val.extract(py))
                    }
                } else {
                    // an empty set never contains anything.
                    Ok(false)
                }
            }
        }
    };
}

// ---------------------------------------------------------------------------

#[pyclass(gc, module = "nanoset")]
#[derive(Debug, Default)]
/// A set that has lower memory footprint if it is empty.
pub struct NanoSet {
    inner: Option<PyObject>,
}

common_impl!(NanoSet);

#[pyproto]
impl PyGCProtocol for NanoSet {
    fn __traverse__(&'p self, visit: PyVisit) -> Result<(), PyTraverseError> {
        match self.inner {
            Some(ref obj) => visit.call(obj),
            None => Ok(()),
        }
    }

    fn __clear__(&'p mut self) {
        if let Some(obj) = self.inner.take() {
            let gil = Python::acquire_gil();
            gil.python().release(obj)
        }
    }
}

// ---------------------------------------------------------------------------

#[pyclass(module = "nanoset")]
#[derive(Debug, Default)]
/// A set that has lower memory footprint if it is empty.
pub struct PicoSet {
    inner: Option<PyObject>,
}

common_impl!(PicoSet);

// ---------------------------------------------------------------------------

#[cfg_attr(feature = "extension-module", pymodule(nanoset))]
pub fn init(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<NanoSet>()?;
    m.add_class::<PicoSet>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", env!("CARGO_PKG_AUTHORS").replace(':', "\n"))?;
    m.add("__build__", pyo3_built::pyo3_built!(py, built))?;

    let cabc = py.import("collections.abc")?;
    let set = cabc.get("Set")?.to_object(py);
    set.call_method1(
        py,
        "register",
        (<NanoSet as pyo3::type_object::PyTypeObject>::type_object(),),
    )?;
    set.call_method1(
        py,
        "register",
        (<PicoSet as pyo3::type_object::PyTypeObject>::type_object(),),
    )?;
    let mutset = cabc.get("MutableSet")?.to_object(py);
    mutset.call_method1(
        py,
        "register",
        (<NanoSet as pyo3::type_object::PyTypeObject>::type_object(),),
    )?;
    mutset.call_method1(
        py,
        "register",
        (<PicoSet as pyo3::type_object::PyTypeObject>::type_object(),),
    )?;

    Ok(())
}
