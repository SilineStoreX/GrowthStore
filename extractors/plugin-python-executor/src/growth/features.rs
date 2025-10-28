use std::sync::{Arc, Mutex};

use chimes_store_core::{dbs::get_growth_invoke_holder, service::invoker::InvocationContext};
use pyo3::{prelude::*, types::PyList, IntoPyObjectExt};
use rbatis::Page;
use serde::Serialize;
use serde_json::Value;
use pyo3::exceptions::PyValueError;

#[pyclass]
pub struct PyInvocationContext {
  pub ctx: Arc<Mutex<InvocationContext>>
}

#[pymethods]
impl PyInvocationContext {

    #[new]
    pub fn py_new() -> Self {
      Self { 
        ctx: InvocationContext::arcnew()
      }
    }

    pub fn get<'py>(&self, py: Python<'py>, name: &str) -> Option<Bound<'py, PyAny>> {
      // self.ctx.lock().unwrap().get(name).
      if let Some(t) = self.ctx.lock().unwrap().get_(name) {
        if let Some(bx) = t.downcast_ref::<Option<Value>>() {
            return convert_json_to_pyobject(py, bx).map(Some).unwrap_or(None);
        }
        if let Some(bx) = t.downcast_ref::<Value>() {
            return convert_json_to_pyobject(py, bx).map(Some).unwrap_or(None);
        }
        if let Some(bx) = t.downcast_ref::<Vec<Value>>() {
            return convert_json_to_pyobject(py, bx).map(Some).unwrap_or(None);
        }
        if let Some(bx) = t.downcast_ref::<Page<Value>>() {
            return convert_json_to_pyobject(py, bx).map(Some).unwrap_or(None);
        }
      };
      None
    }


    pub fn set_failed(&self) {
        self.ctx.lock().unwrap().set_failed();
    }

    pub fn get_return<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyAny>> {
        self.get(py, "RETURN_VALUE")
    }

    pub fn get_status(&self) -> i64 {
        // Self::get(ctx, "RETURN_STATUS")
        self.ctx.lock().unwrap().get_status()
    }

    pub fn get_message(&self) -> String {
        self.ctx.lock().unwrap().get_message()
    }

    pub fn set_status(&self, val: i64) {
        self.ctx.lock()
            .unwrap()
            .set_status(val);
    }

    pub fn set_message(&self, val: &str) {
        let msg = val.to_string();
        self.ctx.lock().unwrap().set_message(&msg);
    }

    pub fn get_return_rawdata(&self) -> bool {
        self.ctx.lock().unwrap().get_return_rawdata()
    }

    pub fn set_return_rawdata(&self, val: bool) {
        self.ctx.lock()
            .unwrap()
            .set_return_rawdata(val);
    }

    pub fn get_response_xml(&self) -> bool {
        self.ctx.lock().unwrap().get_response_xml()
    }

    pub fn set_response_xml(&self, val: bool) {
        self.ctx.lock()
            .unwrap()
            .set_response_xml(val);
    }

    pub fn set<'py>(&self, py: Python<'py>, name: &str, val: &Bound<'py, PyAny>) {
        if let Ok(value) = pyobject_to_json(py, val) {
          self.ctx.lock().unwrap().insert(name, value);
        }
    }

    pub fn set_return<'py>(&self, py: Python<'py>, val: &Bound<'py, PyAny>) {
      self.set(py, "RETURN_VALUE", val);
    }


    pub fn get_string(&self, name: &str) -> String {
        if let Ok(t) = self.ctx.lock().unwrap().get::<String>(name) {
            t.clone()
        } else {
            String::new()
        }
    }

    pub fn get_i64(&self, name: &str) -> i64 {
        if let Ok(t) = self.ctx.lock().unwrap().get::<i64>(name) {
            *t
        } else {
            0i64
        }
    }

    pub fn get_u64(&self, name: &str) -> u64 {
        if let Ok(t) = self.ctx.lock().unwrap().get::<u64>(name) {
            *t
        } else {
            0u64
        }
    }

    pub fn get_bool(&self, name: &str) -> bool {
        if let Ok(t) = self.ctx.lock().unwrap().get::<bool>(name) {
            *t
        } else {
            false
        }
    }

    pub fn get_hook_uri(&self) -> String {
        if let Ok(t) = self.ctx.lock().unwrap().get::<String>("HOOK_HANDLE_URI") {
            t.clone()
        } else {
            String::new()
        }
    }

    pub fn set_username(&self, username: &str, userid: &str) {
        self.ctx.lock().unwrap().set_current_user(username, Some(userid));
    }

    pub fn set_username_withid(&self, username: &str, userid: i64) {
        if userid == 0 {
          self.ctx.lock().unwrap().set_current_user(username, None);
        } else {
          self.ctx.lock().unwrap().set_current_user(username, Some(&format!("{userid}")));
        }
    }

    pub fn get_username(&self) -> Option<String> {
        self.ctx.lock().unwrap().get_current_user()
    }

    pub fn get_userid(&self) -> Option<String> {
        self.ctx.lock().unwrap().get_current_userid()
    }


    /**
     * 获取当前用户的Authorization Token, JwtToken
     */
    pub fn get_authorization(&self) -> Option<String> {
        self.ctx.lock().unwrap().get_current_authorization()
    }    

    pub fn lock<'py>(
        &self,
        py: Python<'py>,
        ns: &str,
        key: &str,
        expr: i64,
    ) -> PyResult<Bound<'py, PyAny>>  {
        match self.ctx.lock().unwrap().lock(ns, key, expr) {
            Ok(t) => t.into_bound_py_any(py),
            Err(err) => Err(PyValueError::new_err(format!("Could not lock this key, {err:?}"))),
        }
    }

    pub fn lock_ns<'py>(
        &self,
        py: Python<'py>,
        ns: &str,
        key: &str,
    ) -> PyResult<Bound<'py, PyAny>>  {
        self.lock(py, ns, key, 30)
    }

    pub fn unlock<'py>(
        &self,
        py: Python<'py>,
        ns: &str,
        key: &str,
    ) -> PyResult<Bound<'py, PyAny>>  {
        match self.ctx.lock().unwrap().unlock(ns, key) {
            Ok(t) => t.into_bound_py_any(py),
            Err(err) => Err(PyValueError::new_err(format!("Unable to not unlock this key, {err:?}"))),
        }
    }

    pub fn release_connections(&self) {
        get_growth_invoke_holder().release_connections(self.ctx.clone());
    }   

}


#[pyfunction]
pub(crate) fn required(py: Python<'_>, nsuri: &str) -> PyResult<Py<PyAny>> {
  if nsuri.starts_with("object://") {
    let pyobj = PythonStoreObject::py_new(nsuri).into_py_any(py)?;
    Ok(pyobj)
  } else if  nsuri.starts_with("query://") {
    let pyobj = PythonStoreQuery::py_new(nsuri).into_py_any(py)?;
    Ok(pyobj)
  } else {
    let pyobj = PythonStorePlugin::py_new(nsuri).into_py_any(py)?;
    Ok(pyobj)
  }
}

#[pyfunction]
pub(crate) fn json_to_pyobject<'py>(py: Python<'py>, json: &str) -> PyResult<Bound<'py, PyAny>> {
  let pyobj = json.into_pyobject(py)?;
  let json_mod = py.import("json")?;
  json_mod.call_method1("loads", &(pyobj,))
}


pub(crate) fn convert_json_to_pyobject<'py, T>(py: Python<'py>, jsval: &T) -> PyResult<Bound<'py, PyAny>> 
where
    T: ?Sized + Serialize,
{
  let jstext = serde_json::to_string(jsval).map_err(|e| PyValueError::new_err(format!("error to convert to text {e:?}")))?;
  json_to_pyobject(py, &jstext)
}

pub(crate) fn pyobject_to_json<'py>(py: Python<'py>, pyobj: &Bound<'py, PyAny>) -> PyResult<Value> {
  let json_mod = py.import("json")?;
  let json = json_mod.call_method1("dumps", (pyobj,))?.to_string();

  serde_json::from_str(&json)
      .map_err(|e| {
        PyValueError::new_err(format!("args could not convert to json {e:?}."))
      })
}

#[pyfunction]
#[pyo3(name="get_namespace_config")]
pub(crate) fn py_get_namespace_config<'py>(py: Python<'py>, ns: &str) -> PyResult<Bound<'py, PyAny>> {
  match get_growth_invoke_holder().get_namespace_config(ns) {
    Ok(val) => {
      let text = match serde_json::to_string(&val) {
        Ok(tstr) => tstr,
        Err(err) => {
          return Err(PyValueError::new_err(format!("error to convert json {err}")));
        } 
      };
      json_to_pyobject(py, &text)
    },
    Err(err) => {
      Err(PyValueError::new_err(format!("could not get the config value {err}")))
    }
  }
}


#[pyfunction]
#[pyo3(name="get_plugin_config")]
pub(crate) fn py_get_plugin_config<'py>(py: Python<'py>, uri: &str) -> PyResult<Bound<'py, PyAny>> {
  match get_growth_invoke_holder().get_plugin_config(uri) {
    Ok(val) => {
      let text = match serde_json::to_string(&val) {
        Ok(tstr) => tstr,
        Err(err) => {
          return Err(PyValueError::new_err(format!("error to convert json {err}")));
        } 
      };
      json_to_pyobject(py, &text)
    },
    Err(err) => {
      Err(PyValueError::new_err(format!("could not get the config value {err}")))
    }
  }
}


#[pyclass]
pub(crate) struct PythonStoreObject {
  nsobj: String
}

#[pymethods]
impl PythonStoreObject {

    #[new]
    pub fn py_new(nsuri: &str) -> Self {
      Self { 
        nsobj: nsuri.to_string()
      }
    }

    pub fn get_invoke_uri(&self) -> String {
      return self.nsobj.clone()
    }

    pub fn invoke_return_option<'py>(&self, uri: &str, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      let py = dicts.py();
      let listargs = dicts.iter().map(|s| {
        pyobject_to_json(py, &s).unwrap_or(Value::Null)
      }).collect::<Vec<_>>();
      let holder = get_growth_invoke_holder();
      let st_uri = format!("{}#{}", self.nsobj, uri);
      let tx_ctx = ctx.ctx.clone();
      // pin_blockon_async_v2!(async move {
        match holder.invoke_return_option(&st_uri, tx_ctx, &listargs) {
          Ok(t) => {
            let text = match serde_json::to_string(&t) {
              Ok(tstr) => tstr,
              Err(err) => {
                return Err(PyValueError::new_err(format!("error to convert json {err}")));
              } 
            };
            json_to_pyobject(py, &text)
          },
          Err(err) => {
            Err(PyValueError::new_err(format!("Error on execute {err}")))
          }
        }
      // }).map_err(|e| {
      //  println!("Error on {e:?}");
      //  e
      //}).unwrap_or(Err(PyValueError::new_err("could not invoke the request")))
    }

    pub fn invoke_return_page<'py>(&self, uri: &str, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      let py = dicts.py();
      let listargs = dicts.iter().map(|s| {
        pyobject_to_json(py, &s).unwrap_or(Value::Null)
      }).collect::<Vec<_>>();
      let holder = get_growth_invoke_holder();
      let st_uri = format!("{}#{}", self.nsobj, uri);
      let tx_ctx = ctx.ctx.clone();
      // pin_blockon_async_v2!(async move {
        match holder.invoke_return_paged(&st_uri, tx_ctx, &listargs) {
          Ok(t) => {
            let text = match serde_json::to_string(&t) {
              Ok(tstr) => tstr,
              Err(err) => {
                return Err(PyValueError::new_err(format!("error to convert json {err}")));
              } 
            };
            json_to_pyobject(py, &text)
          },
          Err(err) => {
            Err(PyValueError::new_err(format!("Error on execute {err}")))
          }
        }
      // }).map_err(|e| {
      //  println!("Error on {e:?}");
      //  e
      //}).unwrap_or(Err(PyValueError::new_err("could not invoke the request")))
    }    

    pub fn invoke_return_list<'py>(&self, uri: &str, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      let py = dicts.py();
      let listargs = dicts.iter().map(|s| {
        pyobject_to_json(py, &s).unwrap_or(Value::Null)
      }).collect::<Vec<_>>();
      let holder = get_growth_invoke_holder();
      let st_uri = format!("{}#{}", self.nsobj, uri);
      let tx_ctx = ctx.ctx.clone();
      // pin_blockon_async_v2!(async move {
        match holder.invoke_return_vec(&st_uri, tx_ctx, &listargs) {
          Ok(t) => {
            let text = match serde_json::to_string(&t) {
              Ok(tstr) => tstr,
              Err(err) => {
                return Err(PyValueError::new_err(format!("error to convert json {err}")));
              } 
            };
            json_to_pyobject(py, &text)
          },
          Err(err) => {
            Err(PyValueError::new_err(format!("Error on execute {err}")))
          }
        }
      // }).map_err(|e| {
      //  println!("Error on {e:?}");
      //  e
      //}).unwrap_or(Err(PyValueError::new_err("could not invoke the request")))
    }

    pub fn paged_query<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_page("paged_query", ctx, dicts)
    }

    pub fn query<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_list("query", ctx, dicts)
    }

    pub fn select_one<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_option("select", ctx, dicts)
    }

    pub fn fine_one<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_option("find_one", ctx, dicts)
    }

    pub fn insert<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_option("insert", ctx, dicts)
    }

    pub fn update<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_option("update", ctx, dicts)
    }

    pub fn upsert<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_option("upsert", ctx, dicts)
    }

    pub fn save_batch<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_option("save_batch", ctx, dicts)
    }

    pub fn update_by<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_option("update_by", ctx, dicts)
    }

    pub fn delete<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_option("delete", ctx, dicts)
    }

    pub fn delete_by<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_option("delete_by", ctx, dicts)
    }
}

#[pyclass]
pub(crate) struct PythonStoreQuery {
  nsobj: String
}

#[pymethods]
impl PythonStoreQuery {

    #[new]
    pub fn py_new(nsuri: &str) -> Self {
      Self { 
        nsobj: nsuri.to_string()
      }
    }

    pub fn invoke_return_option<'py>(&self, uri: &str, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      let py = dicts.py();
      let listargs = dicts.iter().map(|s| {
        pyobject_to_json(py, &s).unwrap_or(Value::Null)
      }).collect::<Vec<_>>();
      let holder = get_growth_invoke_holder();
      let st_uri = format!("{}#{}", self.nsobj, uri);
      let tx_ctx = ctx.ctx.clone();
      // pin_blockon_async_v2!(async move {
        match holder.invoke_return_option(&st_uri, tx_ctx, &listargs) {
          Ok(t) => {
            let text = match serde_json::to_string(&t) {
              Ok(tstr) => tstr,
              Err(err) => {
                return Err(PyValueError::new_err(format!("error to convert json {err}")));
              } 
            };
            json_to_pyobject(py, &text)
          },
          Err(err) => {
            Err(PyValueError::new_err(format!("Error on execute {err}")))
          }
        }
      // }).map_err(|e| {
      //  println!("Error on {e:?}");
      //  e
      //}).unwrap_or(Err(PyValueError::new_err("could not invoke the request")))
    }

    pub fn invoke_return_page<'py>(&self, uri: &str, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      let py = dicts.py();
      let listargs = dicts.iter().map(|s| {
        pyobject_to_json(py, &s).unwrap_or(Value::Null)
      }).collect::<Vec<_>>();
      let holder = get_growth_invoke_holder();
      let st_uri = format!("{}#{}", self.nsobj, uri);

      let tx_ctx = ctx.ctx.clone();
      // pin_blockon_async_v2!(async move {
        match holder.invoke_return_paged(&st_uri, tx_ctx, &listargs) {
          Ok(t) => {
            let text = match serde_json::to_string(&t) {
              Ok(tstr) => tstr,
              Err(err) => {
                return Err(PyValueError::new_err(format!("error to convert json {err}")));
              } 
            };
            json_to_pyobject(py, &text)
          },
          Err(err) => {
            Err(PyValueError::new_err(format!("Error on execute {err}")))
          }
        }
      // }).map_err(|e| {
      //  println!("Error on {e:?}");
      //  e
      //}).unwrap_or(Err(PyValueError::new_err("could not invoke the request")))
    }    

    pub fn invoke_return_list<'py>(&self, uri: &str, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      let py = dicts.py();
      let listargs = dicts.iter().map(|s| {
        pyobject_to_json(py, &s).unwrap_or(Value::Null)
      }).collect::<Vec<_>>();
      let holder = get_growth_invoke_holder();
      let st_uri = format!("{}#{}", self.nsobj, uri);

      let tx_ctx = ctx.ctx.clone();
      // pin_blockon_async_v2!(async move {
        match holder.invoke_return_vec(&st_uri, tx_ctx, &listargs) {
          Ok(t) => {
            let text = match serde_json::to_string(&t) {
              Ok(tstr) => tstr,
              Err(err) => {
                return Err(PyValueError::new_err(format!("error to convert json {err}")));
              } 
            };
            json_to_pyobject(py, &text)
          },
          Err(err) => {

            Err(PyValueError::new_err(format!("Error on execute {err}")))
          }
        }
      // }).map_err(|e| {
      //  println!("Error on {e:?}");
      //  e
      //}).unwrap_or(Err(PyValueError::new_err("could not invoke the request")))
    }

    pub fn direct_query<'py>(&self, uri: &str, ctx: &PyInvocationContext, query: &str, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {

      let py = dicts.py();
      let listargs = dicts.iter().map(|s| {
        pyobject_to_json(py, &s).unwrap_or(Value::Null)
      }).collect::<Vec<_>>();
      let holder = get_growth_invoke_holder();
      let st_uri = format!("{}#{}", self.nsobj, uri);

      let tx_ctx = ctx.ctx.clone();
      // pin_blockon_async_v2!(async move {
        match holder.direct_invoke(&st_uri, tx_ctx, query, &listargs) {
          Ok(t) => {
            let text = match serde_json::to_string(&t) {
              Ok(tstr) => tstr,
              Err(err) => {
                return Err(PyValueError::new_err(format!("error to convert json {err}")));
              } 
            };
            json_to_pyobject(py, &text)
          },
          Err(err) => {
            Err(PyValueError::new_err(format!("Error on execute {err}")))
          }
        }
      // }).map_err(|e| {
      //  println!("Error on {e:?}");
      //  e
      //}).unwrap_or(Err(PyValueError::new_err("could not invoke the request")))
    }

    pub fn direct_query_v2<'py>(&self, uri: &str, ctx: &PyInvocationContext, query: Bound<'py, PyAny>, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {

      let py = dicts.py();
      let listargs = dicts.iter().map(|s| {
        pyobject_to_json(py, &s).unwrap_or(Value::Null)
      }).collect::<Vec<_>>();
      let jsquery = pyobject_to_json(py, &query).unwrap_or(Value::Null);
      let holder = get_growth_invoke_holder();
      let st_uri = format!("{}#{}", self.nsobj, uri);
      let tx_ctx = ctx.ctx.clone();
      // pin_blockon_async_v2!(async move {
        match holder.direct_invoke_v2(&st_uri, tx_ctx, jsquery, &listargs) {
          Ok(t) => {
            let text = match serde_json::to_string(&t) {
              Ok(tstr) => tstr,
              Err(err) => {
                return Err(PyValueError::new_err(format!("error to convert json {err}")));
              } 
            };
            json_to_pyobject(py, &text)
          },
          Err(err) => {
            Err(PyValueError::new_err(format!("Error on execute {err}")))
          }
        }
      // }).map_err(|e| {
      //  println!("Error on {e:?}");
      //  e
      //}).unwrap_or(Err(PyValueError::new_err("could not invoke the request")))
    }

    pub fn find_one<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_option("find_one", ctx, dicts)
    }

    pub fn paged_search<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_page("paged_search", ctx, dicts)
    }

    pub fn search<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_list("search", ctx, dicts)
    }

    pub fn execute<'py>(&self, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      self.invoke_return_option("execute", ctx, dicts)
    }
}


#[pyclass]
pub(crate) struct PythonStorePlugin {
  nsobj: String
}

#[pymethods]
impl PythonStorePlugin {

    #[new]
    pub fn py_new(nsuri: &str) -> Self {
      Self { 
        nsobj: nsuri.to_string()
      }
    }

    pub fn invoke_return_option<'py>(&self, uri: &str, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      let py = dicts.py();
      let listargs = dicts.iter().map(|s| {
        pyobject_to_json(py, &s).unwrap_or(Value::Null)
      }).collect::<Vec<_>>();
      let holder = get_growth_invoke_holder();
      let st_uri = format!("{}#{}", self.nsobj, uri);
      let tx_ctx = ctx.ctx.clone();
      // pin_blockon_async_v2!(async move {
        match holder.invoke_return_option(&st_uri, tx_ctx, &listargs) {
          Ok(t) => {
            let text = match serde_json::to_string(&t) {
              Ok(tstr) => tstr,
              Err(err) => {
                return Err(PyValueError::new_err(format!("error to convert json {err}")));
              } 
            };
            json_to_pyobject(py, &text)
          },
          Err(err) => {
            Err(PyValueError::new_err(format!("Error on execute {err}")))
          }
        }
      // }).map_err(|e| {
      //  println!("Error on {e:?}");
      //  e
      //}).unwrap_or(Err(PyValueError::new_err("could not invoke the request")))
    }

    pub fn invoke_return_page<'py>(&self, uri: &str, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      let py = dicts.py();
      let listargs = dicts.iter().map(|s| {
        pyobject_to_json(py, &s).unwrap_or(Value::Null)
      }).collect::<Vec<_>>();
      let holder = get_growth_invoke_holder();
      let st_uri = format!("{}#{}", self.nsobj, uri);
      let tx_ctx = ctx.ctx.clone();
      // pin_blockon_async_v2!(async move {
        match holder.invoke_return_paged(&st_uri, tx_ctx, &listargs) {
          Ok(t) => {
            let text = match serde_json::to_string(&t) {
              Ok(tstr) => tstr,
              Err(err) => {
                return Err(PyValueError::new_err(format!("error to convert json {err}")));
              } 
            };
            json_to_pyobject(py, &text)
          },
          Err(err) => {
            Err(PyValueError::new_err(format!("Error on execute {err}")))
          }
        }
      // }).map_err(|e| {
      //  println!("Error on {e:?}");
      //  e
      //}).unwrap_or(Err(PyValueError::new_err("could not invoke the request")))
    }    

    pub fn invoke_return_list<'py>(&self, uri: &str, ctx: &PyInvocationContext, dicts: &Bound<'py, PyList>) -> PyResult<Bound<'py, PyAny>> {
      let py = dicts.py();
      let listargs = dicts.iter().map(|s| {
        pyobject_to_json(py, &s).unwrap_or(Value::Null)
      }).collect::<Vec<_>>();
      let holder = get_growth_invoke_holder();
      let st_uri = format!("{}#{}", self.nsobj, uri);

      let tx_ctx = ctx.ctx.clone();
      // pin_blockon_async_v2!(async move {
        match holder.invoke_return_vec(&st_uri, tx_ctx, &listargs) {
          Ok(t) => {
            let text = match serde_json::to_string(&t) {
              Ok(tstr) => tstr,
              Err(err) => {
                return Err(PyValueError::new_err(format!("error to convert json {err}")));
              } 
            };
            json_to_pyobject(py, &text)
          },
          Err(err) => {
            Err(PyValueError::new_err(format!("Error on execute {err}")))
          }
        }
      // }).map_err(|e| {
      //  println!("Error on {e:?}");
      //  e
      //}).unwrap_or(Err(PyValueError::new_err("could not invoke the request")))
    }    
}
