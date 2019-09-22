#![feature(doc_cfg, external_doc)]
#![doc(include = "../README.md")]
#![cfg_attr(feature = "extension-module", crate_type = "cdylib")]

extern crate pyo3;

#[cfg(feature = "extension-module")]
extern crate pyo3_built;
#[cfg(feature = "extension-module")]
mod built;

use std::ops::Deref;

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
use pyo3::types::PyFrozenSet;
use pyo3::types::PyIterator;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
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
            fn __new__(obj: &PyRawObject, iterable: Option<&PyAny>) -> PyResult<()> {
                let mut val = Self::new();
                val.__init__(iterable)?;
                obj.init(val);
                Ok(())
            }

            fn __init__(&mut self, iterable: Option<&PyAny>) -> PyResult<()> {
                if let Some(it) = iterable {
                    let gil = Python::acquire_gil();
                    let py = gil.python();
                    *self = Self::try_from_any(py, it)?;
                } else {
                    self.inner = None;
                }
                Ok(())
            }

            // fn __getstate__(&self) -> PyResult<PyObject> {
            //     let gil = Python::acquire_gil();
            //     let py = gil.python();
            //     match self.inner {
            //         None => Ok(py.None()),
            //         Some(ref set) => Ok(set.clone_ref(py)),
            //     }
            // }
            //
            // fn __setstate__(&mut self, state: PyObject) {
            //     unimplemented!()
            // }

            fn __reduce__(&self) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                let ty = <$cls as pyo3::type_object::PyTypeObject>::type_object();

                match self.inner {
                    None => Ok((ty, PyTuple::empty(py)).to_object(py)),
                    Some(ref set) => Ok((ty, (set.call_method0(py, "copy")?,)).to_object(py)),
                }
            }

            fn add(&mut self, item: &PyAny) -> PyResult<()> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                if self.inner.is_none() {
                    self.inner = Some(PySet::empty(py)?.to_object(py));
                }

                self.inner.as_ref().unwrap().cast_as::<PySet>(py)?.add(item)
            }

            fn clear(&mut self) -> PyResult<()> {
                if let Some(inner) = self.inner.take() {
                    Python::acquire_gil().python().release(inner)
                };

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
            fn difference(&self, others: &PyTuple) -> PyResult<Self> {
                let py = others.py();
                if let Some(ref obj) = self.inner {
                    // substracting from non-empty set may give an
                    // empty set --> we must use `try_from_obj`.
                    obj.call_method1(py, "difference", others)
                        .and_then(|s| Self::try_from_obj(py, s))
                } else {
                    // we still need to typecheck the arguments to
                    // comply with Python issue #37219
                    for arg in others.iter() {
                        if let Err(e) = PyIterator::from_object(py, arg) {
                            return Err(e.into());
                        }
                    }

                    // substracting from an empty set always gives
                    // an empty set --> we can use `new`.
                    Ok(Self::new())
                }
            }

            #[args(others = "*")]
            fn difference_update(&mut self, others: &PyTuple) -> PyResult<()> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                if let Some(ref inner) = self.inner {
                    // the difference is nonempty if the set is nonempty
                    inner.call_method1(py, "difference_update", others)?;

                    // the set may have been emptied, so we need
                    // to update `self.inner` to maintain the
                    // invariant.
                    if inner.cast_as::<PySet>(py).unwrap().is_empty() {
                        self.inner = None;
                    }
                } else {
                    // we still need to typecheck the arguments to
                    // comply with Python issue #37219
                    for arg in others.iter() {
                        if let Err(e) = PyIterator::from_object(py, arg) {
                            return Err(e.into());
                        }
                    }
                }

                Ok(())
            }

            fn discard(&mut self, elem: &PyAny) -> PyResult<()> {
                if let Some(ref set) = self.inner {
                    let gil = Python::acquire_gil();
                    set.call_method1(gil.python(), "discard", (elem,))?;
                }
                Ok(())
            }

            #[args(others = "*")]
            fn intersection(&self, others: &PyTuple) -> PyResult<Self> {
                let py = others.py();
                if let Some(ref obj) = self.inner {
                    // intersecting an non-empty set may give an
                    // empty set --> we must use `try_from_obj`.
                    obj.call_method1(py, "intersection", others)
                        .and_then(|s| Self::try_from_obj(py, s))
                } else {
                    // intersecting an empty set always gives
                    // an empty set --> we can use `new`.
                    Ok(Self::new())
                }
            }

            #[args(others = "*")]
            fn intersection_update(&mut self, others: &PyTuple) -> PyResult<()> {
                // the intersection is nonempty if the set is nonempty
                if let Some(ref inner) = self.inner {
                    let gil = Python::acquire_gil();
                    let py = gil.python();
                    inner.call_method1(py, "intersection_update", others)?;

                    // the set may have been emptied, so we need
                    // to update `self.inner` to maintain the
                    // invariant.
                    if inner.cast_as::<PySet>(py).unwrap().is_empty() {
                        self.inner = None;
                    }
                }

                Ok(())
            }

            fn isdisjoint(&self, other: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match self.inner {
                    None => Ok(true.to_object(py)),
                    Some(ref obj) => obj.call_method1(py, "isdisjoint", (other,)),
                }
            }

            fn issubset(&self, other: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match self.inner {
                    None => Ok(true.to_object(py)),
                    Some(ref obj) => obj.call_method1(py, "issubset", (other,)),
                }
            }

            fn issuperset(&self, other: &PyAny) -> PyResult<bool> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                let mut it = PyIterator::from_object(py, other)?;
                match self.inner {
                    None => Ok(it.next().is_none()),
                    Some(ref inner) => {
                        let set = inner.cast_as::<PySet>(py).unwrap();
                        for item in it {
                            if !set.contains(item?)? {
                                return Ok(false);
                            }
                        }
                        Ok(true)
                    }
                }
            }

            fn pop(&mut self) -> PyResult<PyObject> {
                if let Some(ref inner) = self.inner {
                    let gil = Python::acquire_gil();
                    let set = inner.cast_as::<PySet>(gil.python())?;
                    if let Some(item) = set.pop() {
                        if set.len() == 0 {
                            self.inner = None;
                        }
                        return Ok(item);
                    }
                }

                KeyError::into("pop from an empty set")
            }

            fn remove(&mut self, item: &PyAny) -> PyResult<()> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                if let Some(ref inner) = self.inner {
                    // `set2.remove(set1)` actually does for
                    // `set2.remove(frozenset(set1))`, so we have to check if
                    // `set1` is `NanoSet` to reproduce that behaviour.
                    if let Ok(ref other) = item.cast_as::<$cls>() {
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
                        self.inner = None
                    }

                    Ok(())
                } else {
                    KeyError::into(item.to_object(py))
                }
            }

            fn symmetric_difference(&self, other: &PyAny) -> PyResult<Self> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                if let Some(ref obj) = self.inner {
                    // substracting from non-empty set may give an
                    // empty set --> we must use `try_from_obj`.
                    obj.call_method1(py, "symmetric_difference", (other,))
                        .and_then(|obj| Self::try_from_obj(py, obj))
                } else {
                    // all items of `other` are not in the set
                    // since it is empty, but `other` may be empty
                    // as well --> we must use `try_from_obj`.
                    Self::try_from_any(py, other)
                }
            }

            #[args(others = "*")]
            fn symmetric_difference_update(&self, others: &PyTuple) -> PyResult<()> {
                if let Some(ref inner) = self.inner {
                    let gil = Python::acquire_gil();
                    let py = gil.python();
                    inner.call_method1(py, "symmetric_difference_update", others)?;
                }

                Ok(())
            }

            #[args(others = "*")]
            fn union(&self, others: &PyTuple) -> PyResult<Self> {
                let py = others.py();
                if let Some(ref inner) = self.inner {
                    // The set was not empty before, so it will not
                    // be after the union --> we can use `from_set`.
                    inner.call_method1(py, "union", others).map(Self::from_set)
                } else {
                    // The set was empty before, but it may still be
                    // after the union --> we must use `try_from_obj`.
                    let s = PySet::empty(py)?.to_object(py);
                    s.call_method1(py, "union", others)
                        .and_then(|set| Self::try_from_obj(py, set))
                }
            }

            #[args(others = "*")]
            fn update(&mut self, others: &PyTuple) -> PyResult<()> {
                for other in others.iter() {
                    match self.inner {
                        None => {
                            *self = Self::try_from_any(others.py(), other)?;
                        }
                        Some(ref inner) => {
                            inner.call_method1(others.py(), "update", (other,))?;
                        }
                    }
                }

                Ok(())
            }
        }

        #[pyproto]
        impl PyIterProtocol for $cls {
            fn __iter__(slf: PyRefMut<'p, Self>) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match slf.deref().inner {
                    None => PyTuple::empty(py)
                        .to_object(py)
                        .call_method0(py, "__iter__"),
                    Some(ref inner) => inner.call_method0(py, "__iter__"),
                }
            }
        }

        #[pyproto]
        impl PyNumberProtocol for $cls {
            fn __and__(lhs: &Self, rhs: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                if rhs.cast_as::<PySet>().is_err()
                    && rhs.cast_as::<PyFrozenSet>().is_err()
                    && rhs.cast_as::<$cls>().is_err()
                {
                    return Ok(py.NotImplemented());
                }

                let args: Py<PyTuple> = (rhs,).into_py(gil.python());
                lhs.intersection(&args.as_ref(py))
                    .and_then(|s| Py::new(py, s))
                    .map(PyObject::from)
            }

            fn __sub__(lhs: &Self, rhs: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                if rhs.cast_as::<PySet>().is_err()
                    && rhs.cast_as::<PyFrozenSet>().is_err()
                    && rhs.cast_as::<$cls>().is_err()
                {
                    return Ok(py.NotImplemented());
                }

                let args: Py<PyTuple> = (rhs,).into_py(gil.python());
                lhs.difference(&args.as_ref(py))
                    .and_then(|s| Py::new(py, s))
                    .map(PyObject::from)
            }

            fn __or__(lhs: &Self, rhs: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                if rhs.cast_as::<PySet>().is_err()
                    && rhs.cast_as::<PyFrozenSet>().is_err()
                    && rhs.cast_as::<$cls>().is_err()
                {
                    return Ok(py.NotImplemented());
                }

                let args: Py<PyTuple> = (rhs,).into_py(gil.python());
                lhs.union(&args.as_ref(py))
                    .and_then(|s| Py::new(py, s))
                    .map(PyObject::from)
            }

            fn __xor__(lhs: &Self, rhs: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();

                if rhs.cast_as::<PySet>().is_err()
                    && rhs.cast_as::<PyFrozenSet>().is_err()
                    && rhs.cast_as::<$cls>().is_err()
                {
                    return Ok(py.NotImplemented());
                }

                lhs.symmetric_difference(rhs)
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

                if let Ok(other) = obj.cast_as::<Self>() {
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
                } else if let Ok(_other) = obj.cast_as::<PyFrozenSet>() {
                    // unimplemented!(concat!(stringify!($cls), ".__cmp__(frozenset)"));
                    Ok(py.NotImplemented())
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
                    if let Ok(ref other) = item.cast_as::<$cls>() {
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

#[cfg(feature = "extension-module")]
#[pymodule]
fn nanoset(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<NanoSet>()?;
    m.add_class::<PicoSet>()?;
    m.add("__build__", pyo3_built::pyo3_built!(py, built))?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", env!("CARGO_PKG_AUTHORS").replace(':', "\n"))?;

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

    Ok(())
}
