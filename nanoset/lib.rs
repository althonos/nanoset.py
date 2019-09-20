extern crate pyo3;
extern crate pyo3_built;

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
use pyo3::types::PyIterator;
use pyo3::types::PySet;
use pyo3::types::PyString;
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
                    None => Ok(ty.to_object(py)),
                    Some(ref set) => Ok(
                        (
                            ty.to_object(py),
                            (set.call_method0(py, "copy")?,)
                        )
                            .to_object(py)
                    )
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

            fn difference(&self, other: &PyAny) -> PyResult<Self> {
                match self.inner {
                    None => Ok(Self::new()),
                    Some(ref obj) => {
                        let gil = Python::acquire_gil();
                        let py = gil.python();
                        obj.call_method1(py, "difference", (other,))
                            .and_then(|obj| Self::try_from_obj(py, obj))
                    }
                }
            }

            fn intersection(&self, other: &PyAny) -> PyResult<Self> {
                match self.inner {
                    None => Ok(Self::new()),
                    Some(ref obj) => {
                        let gil = Python::acquire_gil();
                        let py = gil.python();
                        obj.call_method1(py, "intersection", (other,))
                            .and_then(|obj| Self::try_from_obj(py, obj))
                    }
                }
            }

            fn isdisjoint(&self, other: &PyAny) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match self.inner {
                    None => Ok(true.to_object(py)),
                    Some(ref obj) => obj.call_method1(py, "isdisjoint", (other,)),
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

            fn symmetric_difference(&self, other: &PyAny) -> PyResult<Self> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match self.inner {
                    None => Self::try_from_any(py, other),
                    Some(ref obj) => {
                        obj.call_method1(py, "symmetric_difference", (other,))
                            .and_then(|obj| Self::try_from_obj(py, obj))
                    }
                }
            }

            fn union(&self, other: &PyAny) -> PyResult<Self> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match self.inner {
                    None => Self::try_from_any(py, other),
                    Some(ref obj) => {
                        obj.call_method1(py, "union", (other,))
                            .map(Self::from_set)
                    }
                }
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
            fn __and__(lhs: &Self, rhs: &PyAny) -> PyResult<Self> {
                lhs.intersection(rhs)
            }

            fn __sub__(lhs: &Self, rhs: &PyAny) -> PyResult<Self> {
                lhs.difference(rhs)
            }

            fn __or__(lhs: &Self, rhs: &PyAny) -> PyResult<Self> {
                lhs.union(rhs)
            }

            fn __xor__(lhs: &Self, rhs: &PyAny) -> PyResult<Self> {
                lhs.symmetric_difference(rhs)
            }
        }

        #[pyproto]
        impl PyObjectProtocol for $cls {
            fn __repr__(&self) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match self.inner {
                    None => {
                        let s = concat!(stringify!($cls), "()");
                        Ok(s.to_object(py))
                    }
                    Some(ref inner) => {
                        let s = PyString::new(py, concat!(stringify!($cls), "({})"));
                        s.to_object(py).call_method1(py, "format", (inner,))
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
                match self.inner {
                    None => Ok(false),
                    Some(ref obj) => {
                        let gil = Python::acquire_gil();
                        let set = obj.cast_as::<PySet>(gil.python())?;
                        set.contains(item)
                    }
                }
            }
        }
    };
}

// ---------------------------------------------------------------------------

#[pyclass(gc, module="nanoset")]
#[derive(Debug, Default)]
/// A set that has lower memory footprint if it is empty.
struct NanoSet {
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

#[pyclass(module="nanoset")]
#[derive(Debug, Default)]
/// A set that has lower memory footprint if it is empty.
struct PicoSet {
    inner: Option<PyObject>,
}

common_impl!(PicoSet);

// ---------------------------------------------------------------------------

#[pymodule]
fn nanoset(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<NanoSet>()?;
    m.add_class::<PicoSet>()?;
    m.add("__build__", pyo3_built::pyo3_built!(py, built))?;
    Ok(())
}
