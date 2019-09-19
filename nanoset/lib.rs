extern crate pyo3;

use std::ops::Deref;

use pyo3::prelude::*;
use pyo3::PyNativeType;
use pyo3::exceptions::KeyError;
use pyo3::class::PyGCProtocol;
use pyo3::class::PyIterProtocol;
use pyo3::class::PyObjectProtocol;
use pyo3::class::PySequenceProtocol;
use pyo3::gc::PyVisit;
use pyo3::gc::PyTraverseError;
use pyo3::types::PyAny;
use pyo3::types::PyIterator;
use pyo3::types::PySet;
use pyo3::types::PyTuple;
use pyo3::types::PyString;


// --- Common implementation -------------------------------------------------

macro_rules! common_impl {
    ($cls:ty) => {
        impl $cls {
            pub fn new() -> Self {
                Self::default()
            }

            pub fn from_object(obj: PyObject) -> Self {
                Self {
                    inner: Some(obj)
                }
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
                match iterable {
                    None => obj.init(Self::new()),
                    Some(it) => {
                        let py = obj.py();
                        let iterator = PyIterator::from_object(py, it)?;
                        let items: PyResult<Vec<&PyAny>> = iterator.collect();
                        let res = items?;
                        if res.is_empty() {
                            obj.init(Self::new());
                        } else {
                            let set = PySet::new(py, res.as_slice())?;
                            obj.init(Self::from_object(set.to_object(py)));
                        }
                    }
                };

                Ok(())
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
                        inner.call_method0(py, "__iter__").map(Self::from_object)
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
        }

        #[pyproto]
        impl PyIterProtocol for $cls {
            fn __iter__(slf: PyRefMut<'p, Self>) -> PyResult<PyObject> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match slf.deref().inner {
                    None => PyTuple::empty(py).to_object(py).call_method0(py, "__iter__"),
                    Some(ref inner) => inner.call_method0(py, "__iter__"),
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
        }
    }
}

// ---------------------------------------------------------------------------

#[pyclass(gc)]
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

#[pyclass]
#[derive(Debug, Default)]
/// A set that has lower memory footprint if it is empty.
struct PicoSet {
    inner: Option<PyObject>
}

common_impl!(PicoSet);

// ---------------------------------------------------------------------------

#[pymodule]
fn nanoset(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<NanoSet>()?;
    m.add_class::<PicoSet>()?;
    Ok(())
}
