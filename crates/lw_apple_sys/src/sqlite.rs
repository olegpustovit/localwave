//! System SQLite, with statement lifetimes and rollback kept inside the FFI boundary.
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::marker::PhantomData;
use std::ptr;
use std::rc::Rc;

#[link(name = "sqlite3")]
unsafe extern "C" {
    fn sqlite3_open_v2(
        path: *const c_char,
        db: *mut *mut c_void,
        flags: c_int,
        vfs: *const c_char,
    ) -> c_int;
    fn sqlite3_close(db: *mut c_void) -> c_int;
    fn sqlite3_errmsg(db: *mut c_void) -> *const c_char;
    fn sqlite3_busy_timeout(db: *mut c_void, ms: c_int) -> c_int;
    fn sqlite3_exec(
        db: *mut c_void,
        sql: *const c_char,
        callback: *const c_void,
        user: *mut c_void,
        error: *mut *mut c_char,
    ) -> c_int;
    fn sqlite3_prepare_v2(
        db: *mut c_void,
        sql: *const c_char,
        len: c_int,
        stmt: *mut *mut c_void,
        tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_finalize(stmt: *mut c_void) -> c_int;
    fn sqlite3_bind_parameter_count(stmt: *mut c_void) -> c_int;
    fn sqlite3_bind_null(stmt: *mut c_void, index: c_int) -> c_int;
    fn sqlite3_bind_int64(stmt: *mut c_void, index: c_int, value: i64) -> c_int;
    fn sqlite3_bind_double(stmt: *mut c_void, index: c_int, value: f64) -> c_int;
    fn sqlite3_bind_text(
        stmt: *mut c_void,
        index: c_int,
        value: *const c_char,
        len: c_int,
        destroy: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> c_int;
    fn sqlite3_bind_blob(
        stmt: *mut c_void,
        index: c_int,
        value: *const c_void,
        len: c_int,
        destroy: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> c_int;
    fn sqlite3_step(stmt: *mut c_void) -> c_int;
    fn sqlite3_column_type(stmt: *mut c_void, index: c_int) -> c_int;
    fn sqlite3_column_count(stmt: *mut c_void) -> c_int;
    fn sqlite3_column_int64(stmt: *mut c_void, index: c_int) -> i64;
    fn sqlite3_column_double(stmt: *mut c_void, index: c_int) -> f64;
    fn sqlite3_column_text(stmt: *mut c_void, index: c_int) -> *const u8;
    fn sqlite3_column_blob(stmt: *mut c_void, index: c_int) -> *const c_void;
    fn sqlite3_column_bytes(stmt: *mut c_void, index: c_int) -> c_int;
    fn sqlite3_changes(db: *mut c_void) -> c_int;
    fn sqlite3_last_insert_rowid(db: *mut c_void) -> i64;
}

#[derive(Debug)]
pub struct Error {
    pub code: i32,
    pub message: String,
}
impl Error {
    pub fn invalid(message: &str) -> Self {
        Self {
            code: 21,
            message: message.into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy)]
pub enum Value<'a> {
    Null,
    Integer(i64),
    Real(f64),
    Text(&'a str),
    Blob(&'a [u8]),
}

/// A connection belongs to one worker. Neither the handle nor a statement crosses threads.
pub struct Connection {
    raw: *mut c_void,
    _thread: PhantomData<Rc<()>>,
}

impl Connection {
    pub fn open(path: &str) -> Result<Self> {
        let path = CString::new(path).map_err(|_| Error::invalid("Database path contains NUL"))?;
        let mut raw = ptr::null_mut();
        let status =
            unsafe { sqlite3_open_v2(path.as_ptr(), &mut raw, 2 | 4 | 0x8000, ptr::null()) };
        if raw.is_null() {
            return Err(Error {
                code: status,
                message: "Could not allocate SQLite connection".into(),
            });
        }
        let connection = Self {
            raw,
            _thread: PhantomData,
        };
        connection.check(status)?;
        connection.check(unsafe { sqlite3_busy_timeout(raw, 2000) })?;
        Ok(connection)
    }

    fn check(&self, code: i32) -> Result<()> {
        if code == 0 {
            Ok(())
        } else {
            Err(Error {
                code,
                message: unsafe { CStr::from_ptr(sqlite3_errmsg(self.raw)) }
                    .to_string_lossy()
                    .into_owned(),
            })
        }
    }

    pub fn execute_batch(&self, sql: &str) -> Result<()> {
        let sql = CString::new(sql).map_err(|_| Error::invalid("SQL contains NUL"))?;
        self.check(unsafe {
            sqlite3_exec(
                self.raw,
                sql.as_ptr(),
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        })
    }

    fn prepare(&self, sql: &str) -> Result<Statement<'_>> {
        let sql = CString::new(sql).map_err(|_| Error::invalid("SQL contains NUL"))?;
        let mut raw = ptr::null_mut();
        let mut tail = ptr::null();
        let status = unsafe { sqlite3_prepare_v2(self.raw, sql.as_ptr(), -1, &mut raw, &mut tail) };
        let statement = Statement {
            raw,
            _connection: self,
        };
        self.check(status)?;
        if raw.is_null()
            || !unsafe { CStr::from_ptr(tail) }
                .to_bytes()
                .iter()
                .all(u8::is_ascii_whitespace)
        {
            return Err(Error::invalid("Expected one SQL statement"));
        }
        Ok(statement)
    }

    pub fn execute(&self, sql: &str, parameters: &[Value<'_>]) -> Result<usize> {
        let statement = self.prepare(sql)?;
        statement.bind(parameters)?;
        self.check(match unsafe { sqlite3_step(statement.raw) } {
            101 => 0,
            code => code,
        })?;
        Ok(unsafe { sqlite3_changes(self.raw) } as usize)
    }

    pub fn query(
        &self,
        sql: &str,
        parameters: &[Value<'_>],
        visit: &mut dyn FnMut(&Row<'_>) -> Result<()>,
    ) -> Result<()> {
        let statement = self.prepare(sql)?;
        statement.bind(parameters)?;
        loop {
            match unsafe { sqlite3_step(statement.raw) } {
                100 => visit(&Row {
                    raw: statement.raw,
                    _statement: PhantomData,
                })?,
                101 => return Ok(()),
                code => {
                    self.check(code)?;
                }
            }
        }
    }

    pub fn last_insert_id(&self) -> i64 {
        unsafe { sqlite3_last_insert_rowid(self.raw) }
    }

    pub fn transaction<T>(
        &mut self,
        operation: impl FnOnce(&Connection) -> Result<T>,
    ) -> Result<T> {
        self.execute_batch("BEGIN IMMEDIATE")?;
        let rollback = Rollback(self);
        let result = operation(self)?;
        self.execute_batch("COMMIT")?;
        std::mem::forget(rollback);
        Ok(result)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        unsafe {
            sqlite3_close(self.raw);
        }
    }
}
struct Rollback<'a>(&'a Connection);
impl Drop for Rollback<'_> {
    fn drop(&mut self) {
        let _ = self.0.execute_batch("ROLLBACK");
    }
}
struct Statement<'a> {
    raw: *mut c_void,
    _connection: &'a Connection,
}
impl Drop for Statement<'_> {
    fn drop(&mut self) {
        unsafe {
            sqlite3_finalize(self.raw);
        }
    }
}

impl Statement<'_> {
    fn bind(&self, parameters: &[Value<'_>]) -> Result<()> {
        if unsafe { sqlite3_bind_parameter_count(self.raw) } as usize != parameters.len() {
            return Err(Error::invalid("Wrong number of SQL parameters"));
        }
        for (index, value) in parameters.iter().enumerate() {
            if let Value::Real(value) = value
                && !value.is_finite()
            {
                return Err(Error::invalid("SQLite number must be finite"));
            }
            let index = index as i32 + 1;
            let length =
                |n| i32::try_from(n).map_err(|_| Error::invalid("SQL value exceeds 2 GiB"));
            // SQLITE_STATIC is safe here: parameters outlive stepping and finalization in execute/query.
            let code = unsafe {
                match value {
                    Value::Null => sqlite3_bind_null(self.raw, index),
                    Value::Integer(value) => sqlite3_bind_int64(self.raw, index, *value),
                    Value::Real(value) => sqlite3_bind_double(self.raw, index, *value),
                    Value::Text(value) => sqlite3_bind_text(
                        self.raw,
                        index,
                        value.as_ptr().cast(),
                        length(value.len())?,
                        None,
                    ),
                    Value::Blob(value) => sqlite3_bind_blob(
                        self.raw,
                        index,
                        value.as_ptr().cast(),
                        length(value.len())?,
                        None,
                    ),
                }
            };
            self._connection.check(code)?;
        }
        Ok(())
    }
}

pub struct Row<'a> {
    raw: *mut c_void,
    _statement: PhantomData<&'a Statement<'a>>,
}
impl Row<'_> {
    fn kind(&self, column: i32) -> Result<i32> {
        if column < 0 || column >= unsafe { sqlite3_column_count(self.raw) } {
            return Err(Error::invalid("SQLite column index is out of bounds"));
        }
        Ok(unsafe { sqlite3_column_type(self.raw, column) })
    }
    pub fn integer(&self, column: i32) -> Result<Option<i64>> {
        match self.kind(column)? {
            5 => Ok(None),
            1 => Ok(Some(unsafe { sqlite3_column_int64(self.raw, column) })),
            _ => Err(Error::invalid("Expected SQLite integer")),
        }
    }
    pub fn real(&self, column: i32) -> Result<Option<f64>> {
        match self.kind(column)? {
            5 => Ok(None),
            1 | 2 => Ok(Some(unsafe { sqlite3_column_double(self.raw, column) })),
            _ => Err(Error::invalid("Expected SQLite number")),
        }
    }
    pub fn text(&self, column: i32) -> Result<Option<&str>> {
        match self.kind(column)? {
            5 => Ok(None),
            3 => {
                let raw = unsafe { sqlite3_column_text(self.raw, column) };
                if raw.is_null() {
                    return Err(Error::invalid("SQLite could not read text"));
                }
                let bytes = unsafe {
                    std::slice::from_raw_parts(raw, sqlite3_column_bytes(self.raw, column) as usize)
                };
                Ok(Some(std::str::from_utf8(bytes).map_err(|_| {
                    Error::invalid("Invalid UTF-8 in database")
                })?))
            }
            _ => Err(Error::invalid("Expected SQLite text")),
        }
    }
    pub fn blob(&self, column: i32) -> Result<Option<&[u8]>> {
        match self.kind(column)? {
            5 => Ok(None),
            4 => {
                let length = unsafe { sqlite3_column_bytes(self.raw, column) } as usize;
                if length == 0 {
                    return Ok(Some(&[]));
                }
                let raw = unsafe { sqlite3_column_blob(self.raw, column) }.cast::<u8>();
                if raw.is_null() {
                    return Err(Error::invalid("SQLite could not read blob"));
                }
                Ok(Some(unsafe { std::slice::from_raw_parts(raw, length) }))
            }
            _ => Err(Error::invalid("Expected SQLite blob")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn values_rollback_validation_and_readonly_failures() {
        let mut db = Connection::open(":memory:").unwrap();
        db.execute_batch("CREATE TABLE t (i INTEGER, r REAL, t TEXT, b BLOB)")
            .unwrap();
        let values = [
            Value::Integer(i64::MAX),
            Value::Real(2.25),
            Value::Text("日本語\0🎧"),
            Value::Blob(&[]),
        ];
        db.execute("INSERT INTO t VALUES (?, ?, ?, ?)", &values)
            .unwrap();
        db.query("SELECT * FROM t", &[], &mut |row| {
            assert_eq!(row.integer(0)?, Some(i64::MAX));
            assert_eq!(row.real(1)?, Some(2.25));
            assert_eq!(row.text(2)?, Some("日本語\0🎧"));
            assert_eq!(row.blob(3)?, Some(&[][..]));
            assert!(row.integer(2).is_err());
            assert!(row.integer(-1).is_err());
            assert!(row.text(4).is_err());
            Ok(())
        })
        .unwrap();
        let result: Result<()> = db.transaction(|db| {
            db.execute("DELETE FROM t", &[])?;
            Err(Error::invalid("Cancel"))
        });
        assert!(result.is_err());
        let mut count = None;
        db.query("SELECT COUNT(*) FROM t", &[], &mut |row| {
            count = row.integer(0)?;
            Ok(())
        })
        .unwrap();
        assert_eq!(count, Some(1));
        assert!(db.execute("SELECT 1; DELETE FROM t", &[]).is_err());
        assert!(db.execute("INSERT INTO t (i) VALUES (?)", &[]).is_err());
        assert!(
            db.execute("UPDATE t SET r=?", &[Value::Real(f64::NAN)])
                .is_err()
        );
        assert!(
            db.execute("UPDATE t SET r=?", &[Value::Real(f64::INFINITY)])
                .is_err()
        );
        db.execute_batch("PRAGMA query_only=ON").unwrap();
        assert_eq!(db.execute("DELETE FROM t", &[]).unwrap_err().code, 8);
    }
}
