extern crate lazy_static;
extern crate nanoset_py;
extern crate pyo3;

use std::path::Path;
use std::sync::Mutex;

use pyo3::Python;
use pyo3::types::PyDict;
use pyo3::types::PyModule;

lazy_static::lazy_static! {
    pub static ref LOCK: Mutex<()> = Mutex::new(());
}

macro_rules! unittest {
    ($name:ident) => {
        #[test]
        fn $name() {

            // load the source code of the unittest
            let source = Path::new(file!()).parent().unwrap();
            let name = Path::new(stringify!($name)).with_extension("py");
            let code = std::fs::read_to_string(source.join(name)).unwrap();

            // acquire Python
            let result = {
                let _l = LOCK.lock().unwrap();
                let gil = Python::acquire_gil();
                let py = gil.python();

                // create a Python module from our rust code with debug symbols
                let module = PyModule::new(py, "nanoset").unwrap();
                nanoset_py::init(py, &module).unwrap();
                py.import("sys")
                    .unwrap()
                    .get("modules")
                    .unwrap()
                    .downcast::<PyDict>()
                    .unwrap()
                    .set_item("nanoset", module)
                    .unwrap();

                // run the test file
                match py.run(&code, None, None) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        e.print(py);
                        Err(())
                    }
                }
            };


            // check the test succeeded
            result.expect("unittest.main failed");
        }
    }
}

unittest!(test_nanoset);
unittest!(test_picoset);
