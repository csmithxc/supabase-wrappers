//! Provides interface types and trait to develop Postgres foreign data wrapper
//!

use crate::FdwRoutine;
use pgx::prelude::{Date, Timestamp};
use pgx::{
    pg_sys::{self, Datum, Oid},
    AllocatedByRust, FromDatum, IntoDatum, JsonB, PgBuiltInOids, PgOid,
};
use std::collections::HashMap;
use std::fmt;
use std::iter::Zip;
use std::slice::Iter;

// fdw system catalog oids
// https://doxygen.postgresql.org/pg__foreign__data__wrapper_8h.html
// https://doxygen.postgresql.org/pg__foreign__server_8h.html
// https://doxygen.postgresql.org/pg__foreign__table_8h.html

/// Constant can be used in [validator](ForeignDataWrapper::validator)
pub const FOREIGN_DATA_WRAPPER_RELATION_ID: pg_sys::Oid = 2328;

/// Constant can be used in [validator](ForeignDataWrapper::validator)
pub const FOREIGN_SERVER_RELATION_ID: pg_sys::Oid = 1417;

/// Constant can be used in [validator](ForeignDataWrapper::validator)
pub const FOREIGN_TABLE_RELATION_ID: pg_sys::Oid = 3118;

/// A data cell in a data row
#[derive(Debug)]
pub enum Cell {
    Bool(bool),
    I8(i8),
    I16(i16),
    F32(f32),
    I32(i32),
    F64(f64),
    I64(i64),
    String(String),
    Date(Date),
    Timestamp(Timestamp),
    Json(JsonB),
}

impl Clone for Cell {
    fn clone(&self) -> Self {
        match self {
            Cell::Bool(v) => Cell::Bool(*v),
            Cell::I8(v) => Cell::I8(*v),
            Cell::I16(v) => Cell::I16(*v),
            Cell::F32(v) => Cell::F32(*v),
            Cell::I32(v) => Cell::I32(*v),
            Cell::F64(v) => Cell::F64(*v),
            Cell::I64(v) => Cell::I64(*v),
            Cell::String(v) => Cell::String(v.clone()),
            Cell::Date(v) => Cell::Date(v.clone()),
            Cell::Timestamp(v) => Cell::Timestamp(v.clone()),
            Cell::Json(v) => Cell::Json(JsonB(v.0.clone())),
        }
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cell::Bool(v) => write!(f, "{}", v),
            Cell::I8(v) => write!(f, "{}", v),
            Cell::I16(v) => write!(f, "{}", v),
            Cell::F32(v) => write!(f, "{}", v),
            Cell::I32(v) => write!(f, "{}", v),
            Cell::F64(v) => write!(f, "{}", v),
            Cell::I64(v) => write!(f, "{}", v),
            Cell::String(v) => write!(f, "'{}'", v),
            Cell::Date(v) => write!(f, "{:?}", v),
            Cell::Timestamp(v) => write!(f, "{:?}", v),
            Cell::Json(v) => write!(f, "{:?}", v),
        }
    }
}

impl IntoDatum for Cell {
    fn into_datum(self) -> Option<Datum> {
        match self {
            Cell::Bool(v) => v.into_datum(),
            Cell::I8(v) => v.into_datum(),
            Cell::I16(v) => v.into_datum(),
            Cell::F32(v) => v.into_datum(),
            Cell::I32(v) => v.into_datum(),
            Cell::F64(v) => v.into_datum(),
            Cell::I64(v) => v.into_datum(),
            Cell::String(v) => v.into_datum(),
            Cell::Date(v) => v.into_datum(),
            Cell::Timestamp(v) => v.into_datum(),
            Cell::Json(v) => v.into_datum(),
        }
    }

    fn type_oid() -> Oid {
        0
    }
}

impl FromDatum for Cell {
    unsafe fn from_polymorphic_datum(datum: Datum, is_null: bool, typoid: Oid) -> Option<Self>
    where
        Self: Sized,
    {
        if is_null {
            return None;
        }
        let oid = PgOid::from(typoid);
        match oid {
            PgOid::BuiltIn(PgBuiltInOids::BOOLOID) => {
                Some(Cell::Bool(bool::from_datum(datum, false).unwrap()))
            }
            PgOid::BuiltIn(PgBuiltInOids::CHAROID) => {
                Some(Cell::I8(i8::from_datum(datum, false).unwrap()))
            }
            PgOid::BuiltIn(PgBuiltInOids::INT2OID) => {
                Some(Cell::I16(i16::from_datum(datum, false).unwrap()))
            }
            PgOid::BuiltIn(PgBuiltInOids::FLOAT4OID) => {
                Some(Cell::F32(f32::from_datum(datum, false).unwrap()))
            }
            PgOid::BuiltIn(PgBuiltInOids::INT4OID) => {
                Some(Cell::I32(i32::from_datum(datum, false).unwrap()))
            }
            PgOid::BuiltIn(PgBuiltInOids::FLOAT8OID) => {
                Some(Cell::F64(f64::from_datum(datum, false).unwrap()))
            }
            PgOid::BuiltIn(PgBuiltInOids::INT8OID) => {
                Some(Cell::I64(i64::from_datum(datum, false).unwrap()))
            }
            PgOid::BuiltIn(PgBuiltInOids::TEXTOID) => {
                Some(Cell::String(String::from_datum(datum, false).unwrap()))
            }
            PgOid::BuiltIn(PgBuiltInOids::DATEOID) => {
                Some(Cell::Date(Date::from_datum(datum, false).unwrap()))
            }
            PgOid::BuiltIn(PgBuiltInOids::TIMESTAMPOID) => Some(Cell::Timestamp(
                Timestamp::from_datum(datum, false).unwrap(),
            )),
            PgOid::BuiltIn(PgBuiltInOids::JSONBOID) => {
                Some(Cell::Json(JsonB::from_datum(datum, false).unwrap()))
            }
            _ => None,
        }
    }
}

/// A data row in a table
///
/// The row contains a column name list and cell list with same number of
/// elements.
#[derive(Debug, Clone, Default)]
pub struct Row {
    /// column names
    pub cols: Vec<String>,

    /// column cell list, should match with cols
    pub cells: Vec<Option<Cell>>,
}

impl Row {
    /// Create an empty row
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a cell with column name to this row
    pub fn push(&mut self, col: &str, cell: Option<Cell>) {
        self.cols.push(col.to_owned());
        self.cells.push(cell);
    }

    /// Return a zipped <column_name, cell> iterator
    pub fn iter(&self) -> Zip<Iter<'_, String>, Iter<'_, Option<Cell>>> {
        self.cols.iter().zip(self.cells.iter())
    }

    /// Remove a cell at the specified index
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut((&String, &Option<Cell>)) -> bool,
    {
        let keep: Vec<bool> = self.iter().map(f).collect();
        let mut iter = keep.iter();
        self.cols.retain(|_| *iter.next().unwrap());
        iter = keep.iter();
        self.cells.retain(|_| *iter.next().unwrap());
    }
}

/// A restiction value used in [`Qual`], either a [`Cell`] or an array of [`Cell`]
#[derive(Debug, Clone)]
pub enum Value {
    Cell(Cell),
    Array(Vec<Cell>),
}

/// Query restrictions, a.k.a conditions in `WHERE` clause
///
/// A Qual defines a simple condition wich can be used by the FDW to restrict the number
/// of the results. Only simple conditions are supported currently.
///
/// ## Examples
///
/// ```sql
/// where id = 1;
/// -- [Qual { field: "id", operator: "=", value: Cell(I32(1)), use_or: false }]
/// ```
///
/// ```sql
/// where id in (1, 2);
/// -- [Qual { field: "id", operator: "=", value: Array([I64(1), I64(2)]), use_or: true }]
/// ```
///
/// ```sql
/// where col is null
/// -- [Qual { field: "col", operator: "is", value: Cell(String("null")), use_or: false }]
/// ```
///
/// ```sql
/// where bool_col
/// -- [Qual { field: "bool_col", operator: "=", value: Cell(Bool(true)), use_or: false }]
/// ```
///
/// ```sql
/// where id > 1 and col = 'foo';
/// -- [
/// --   Qual { field: "id", operator: ">", value: Cell(I32(1)), use_or: false },
/// --   Qual { field: "col", operator: "=", value: Cell(String("foo")), use_or: false }
/// -- ]
/// ```
#[derive(Debug, Clone)]
pub struct Qual {
    pub field: String,
    pub operator: String,
    pub value: Value,
    pub use_or: bool,
}

impl Qual {
    pub fn deparse(&self) -> String {
        if self.use_or {
            "".to_string()
        } else {
            match &self.value {
                Value::Cell(cell) => format!("{} {} {}", self.field, self.operator, cell),
                Value::Array(_) => unreachable!(),
            }
        }
    }
}

/// Query sort, a.k.a `ORDER BY` clause
///
/// ## Examples
///
/// ```sql
/// order by id;
/// -- [Sort { field: "id", field_no: 1, reversed: false, nulls_first: false, collate: None]
/// ```
///
/// ```sql
/// order by id desc;
/// -- [Sort { field: "id", field_no: 1, reversed: true, nulls_first: true, collate: None]
/// ```
///
/// ```sql
/// order by id desc, col;
/// -- [
/// --   Sort { field: "id", field_no: 1, reversed: true, nulls_first: true, collate: None },
/// --   Sort { field: "col", field_no: 2, reversed: false, nulls_first: false, collate: None }
/// -- ]
/// ```
///
/// ```sql
/// order by id collate "de_DE";
/// -- [Sort { field: "col", field_no: 2, reversed: false, nulls_first: false, collate: Some("de_DE") }]
/// ```
#[derive(Debug, Clone, Default)]
pub struct Sort {
    pub field: String,
    pub field_no: usize,
    pub reversed: bool,
    pub nulls_first: bool,
    pub collate: Option<String>,
}

/// Query limit, a.k.a `LIMIT count OFFSET offset` clause
///
/// ## Examples
///
/// ```sql
/// limit 42;
/// -- Limit { count: 42, offset: 0 }
/// ```
///
/// ```sql
/// limit 42 offset 7;
/// -- Limit { count: 42, offset: 7 }
/// ```
#[derive(Debug, Clone, Default)]
pub struct Limit {
    pub count: i64,
    pub offset: i64,
}

/// The Foreign Data Wrapper trait
///
/// This is the main interface for your foreign data wrapper. Required functions
/// are listed below, all the others are optional.
///
/// 1. new
/// 2. begin_scan
/// 3. iter_scan
/// 4. end_scan
///
/// See the module-level document for more details.
///
pub trait ForeignDataWrapper {
    /// Create a FDW instance
    ///
    /// `options` is the key-value pairs defined in `CREATE SERVER` SQL. For example,
    ///
    /// ```sql
    /// create server my_helloworld_server
    ///   foreign data wrapper wrappers_helloworld
    ///   options (
    ///     foo 'bar'
    /// );
    /// ```
    ///
    /// `options` passed here will be a hashmap { 'foo' -> 'bar' }.
    ///
    /// You can do any initalization in this function, like saving connection
    /// info or API url in an variable, but don't do heavy works like database
    /// connection or API call.
    fn new(options: &HashMap<String, String>) -> Self;

    /// Obtain relation size estimates for a foreign table
    ///
    /// Return the expected number of rows and row size (in bytes) by the
    /// foreign table scan.
    ///
    /// [See more details](https://www.postgresql.org/docs/current/fdw-callbacks.html#FDW-CALLBACKS-SCAN).
    fn get_rel_size(
        &mut self,
        _quals: &[Qual],
        _columns: &[String],
        _sorts: &[Sort],
        _limit: &Option<Limit>,
        _options: &HashMap<String, String>,
    ) -> (i64, i32) {
        (0, 0)
    }

    /// Called when begin executing a foreign scan
    ///
    /// - `quals` - `WHERE` clause pushed down
    /// - `columns` - target columns to be queried
    /// - `sorts` - `ORDER BY` clause pushed down
    /// - `limit` - `LIMIT` clause pushed down
    /// - `options` - the options defined when `CREATE FOREIGN TABLE`
    ///
    /// [See more details](https://www.postgresql.org/docs/current/fdw-callbacks.html#FDW-CALLBACKS-SCAN).
    fn begin_scan(
        &mut self,
        quals: &[Qual],
        columns: &[String],
        sorts: &[Sort],
        limit: &Option<Limit>,
        options: &HashMap<String, String>,
    );

    /// Called when fetch one row from the foreign source, returning it in a [`Row`]
    ///
    /// [See more details](https://www.postgresql.org/docs/current/fdw-callbacks.html#FDW-CALLBACKS-SCAN).
    fn iter_scan(&mut self) -> Option<Row>;

    /// Called when restart the scan from the beginning.
    ///
    /// [See more details](https://www.postgresql.org/docs/current/fdw-callbacks.html#FDW-CALLBACKS-SCAN).
    fn re_scan(&mut self) {}

    /// Called when end the scan
    ///
    /// [See more details](https://www.postgresql.org/docs/current/fdw-callbacks.html#FDW-CALLBACKS-SCAN).
    fn end_scan(&mut self);

    /// Called when begin executing a foreign table modification operation.
    ///
    /// - `options` - the options defined when `CREATE FOREIGN TABLE`
    ///
    /// The foreign table must include a `rowid_column` option which specify
    /// the unique identification column of the foreign table to enable data
    /// modification.
    ///
    /// For example,
    ///
    /// ```sql
    /// create foreign table my_foreign_table (
    ///   id bigint,
    ///   name text
    /// )
    ///   server my_server
    ///   options (
    ///     rowid_column 'id'
    ///   );
    /// ```
    ///
    /// [See more details](https://www.postgresql.org/docs/current/fdw-callbacks.html#FDW-CALLBACKS-UPDATE).
    fn begin_modify(&mut self, _options: &HashMap<String, String>) {}

    /// Called when insert one row into the foreign table
    ///
    /// - row - the new row to be inserted
    ///
    /// [See more details](https://www.postgresql.org/docs/current/fdw-callbacks.html#FDW-CALLBACKS-UPDATE).
    fn insert(&mut self, _row: &Row) {}

    /// Called when update one row into the foreign table
    ///
    /// - rowid - the `rowid_column` cell
    /// - new_row - the new row with updated cells
    ///
    /// [See more details](https://www.postgresql.org/docs/current/fdw-callbacks.html#FDW-CALLBACKS-UPDATE).
    fn update(&mut self, _rowid: &Cell, _new_row: &Row) {}

    /// Called when delete one row into the foreign table
    ///
    /// - rowid - the `rowid_column` cell
    ///
    /// [See more details](https://www.postgresql.org/docs/current/fdw-callbacks.html#FDW-CALLBACKS-UPDATE).
    fn delete(&mut self, _rowid: &Cell) {}

    /// Called when end the table update
    ///
    /// [See more details](https://www.postgresql.org/docs/current/fdw-callbacks.html#FDW-CALLBACKS-UPDATE).
    fn end_modify(&mut self) {}

    /// Returns a FdwRoutine for the FDW
    ///
    /// Not to be used directly, use [`wrappers_fdw`](crate::wrappers_fdw) macro instead.
    fn fdw_routine() -> FdwRoutine
    where
        Self: Sized,
    {
        use crate::{modify, scan};
        let mut fdw_routine =
            FdwRoutine::<AllocatedByRust>::alloc_node(pg_sys::NodeTag_T_FdwRoutine);

        // plan phase
        fdw_routine.GetForeignRelSize = Some(scan::get_foreign_rel_size::<Self>);
        fdw_routine.GetForeignPaths = Some(scan::get_foreign_paths::<Self>);
        fdw_routine.GetForeignPlan = Some(scan::get_foreign_plan::<Self>);
        fdw_routine.ExplainForeignScan = Some(scan::explain_foreign_scan::<Self>);

        // scan phase
        fdw_routine.BeginForeignScan = Some(scan::begin_foreign_scan::<Self>);
        fdw_routine.IterateForeignScan = Some(scan::iterate_foreign_scan::<Self>);
        fdw_routine.ReScanForeignScan = Some(scan::re_scan_foreign_scan::<Self>);
        fdw_routine.EndForeignScan = Some(scan::end_foreign_scan::<Self>);

        // modify phase
        fdw_routine.AddForeignUpdateTargets = Some(modify::add_foreign_update_targets);
        fdw_routine.PlanForeignModify = Some(modify::plan_foreign_modify::<Self>);
        fdw_routine.BeginForeignModify = Some(modify::begin_foreign_modify::<Self>);
        fdw_routine.ExecForeignInsert = Some(modify::exec_foreign_insert::<Self>);
        fdw_routine.ExecForeignDelete = Some(modify::exec_foreign_delete::<Self>);
        fdw_routine.ExecForeignUpdate = Some(modify::exec_foreign_update::<Self>);
        fdw_routine.EndForeignModify = Some(modify::end_foreign_modify::<Self>);

        Self::fdw_routine_hook(&mut fdw_routine);
        fdw_routine.into_pg_boxed()
    }

    /// Additional FwdRoutine setup, called by default `Self::fdw_routine()`
    /// after completing its initialization.
    fn fdw_routine_hook(_routine: &mut FdwRoutine<AllocatedByRust>) {}

    /// Validator function for validating options given in `CREATE` and `ALTER`
    /// commands for its foreign data wrapper, as well as foreign servers, user
    /// mappings, and foreign tables using the wrapper.
    ///
    /// [See more details about validator](https://www.postgresql.org/docs/current/fdw-functions.html)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// fn validator(opt_list: Vec<Option<String>>, catalog: Option<pg_sys::Oid>) {
    ///     if let Some(oid) = catalog {
    ///         match oid {
    ///             FOREIGN_DATA_WRAPPER_RELATION_ID => {
    ///                 // check a required option when create foreign data wrapper
    ///                 check_options_contain(&opt_list, "required_option");
    ///             }
    ///             FOREIGN_SERVER_RELATION_ID => {
    ///                 // check option here when create server
    ///             }
    ///             FOREIGN_TABLE_RELATION_ID => {
    ///                 // check option here when create foreign table
    ///             }
    ///             _ => {}
    ///         }
    ///     }
    /// }
    /// ```
    fn validator(_options: Vec<Option<String>>, _catalog: Option<pg_sys::Oid>) {}
}
