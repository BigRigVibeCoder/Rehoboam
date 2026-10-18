//! The environment a program is checked against (§7.4), as plain data with no I/O.
//!
//! Every name a program can use resolves here at compile time: the Registry enums with their
//! named defaults, the entity kinds, every column of the system specs with its entity, key, type,
//! plane and owner, the declaring systems' reads (each with its `neutral` or marked as an identity
//! read), sends, RNG purposes and parameters, the `[commands]` signatures and the registered
//! systems. The tool crate `rh-manifest` builds an `Env` from the Registry and the specs' text (or
//! from a golden-format `[registry]` slice); this crate never reads a file.
//!
//! Kinds, enums, columns and commands are interned to small ids in declaration order, so a value
//! carries no string and two environments built from the same input are equal.
//!
//! REF: `specs/registry/effect_language_v0.md` §3.1, §4.5, §4.7, §7.4; SPR-017 T-01.

use std::collections::{BTreeMap, BTreeSet};

use super::value::Value;

/// An entity kind (`person`, `title`, …): an index into [`Env::kinds`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KindId(pub(crate) u16);

/// A Registry enum: an index into [`Env::enums`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnumId(pub(crate) u16);

/// A column: an index into [`Env::columns`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnId(pub(crate) u32);

/// A command of `[commands]`: an index into [`Env::commands`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId(pub(crate) u32);

/// A language type (§3). The `Lit*` types are the polymorphic literals of §3.2 before resolution;
/// `Rel` is a relation's type, which is not a value (§3.5).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ty {
    /// `bool`.
    Bool,
    /// `int` (`i64`).
    Int,
    /// `q8_8`.
    Q8,
    /// `q16_16`.
    Q16,
    /// A Registry enum.
    Enum(EnumId),
    /// An entity reference of a kind.
    Ent(KindId),
    /// The opaque `event` (§3.1, N3).
    Event,
    /// An `Int` literal not yet resolved.
    LitInt,
    /// A `Decimal` literal not yet resolved.
    LitDec,
    /// `none` not yet given a kind.
    LitNone,
    /// A relation yielding entities of a kind.
    Rel(KindId),
    /// A key tuple (§3.4).
    Tuple(Vec<Ty>),
    /// A bounded list key (§3.4).
    List(Box<Ty>),
}

impl Ty {
    /// Whether the type is a polymorphic literal.
    #[must_use]
    pub fn is_literal(&self) -> bool {
        matches!(self, Self::LitInt | Self::LitDec | Self::LitNone)
    }

    /// Whether the type is `int`, `q8_8` or `q16_16`.
    #[must_use]
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Q8 | Self::Q16)
    }

    /// Whether the type is `q8_8` or `q16_16`.
    #[must_use]
    pub fn is_fixed(&self) -> bool {
        matches!(self, Self::Q8 | Self::Q16)
    }

    /// Whether the type is a tuple or list key.
    #[must_use]
    pub fn is_key(&self) -> bool {
        matches!(self, Self::Tuple(_) | Self::List(_))
    }
}

/// A Registry enum: its values in declaration order and its named default (§3.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumDef {
    /// The enum's name (`sex`, `death_cause`, …).
    pub name: String,
    /// Its values, in declaration order: a value's ordinal is its index.
    pub values: Vec<String>,
    /// The default's index: the value a missing read yields.
    pub default: u32,
}

/// A column as the checker sees it (§7.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColumnDef {
    /// `entity.column`.
    pub name: String,
    /// The table's entity kind.
    pub entity: KindId,
    /// The kind a read takes: the table's own, or `person` for the life-course log.
    pub key: KindId,
    /// The column's language type.
    pub ty: Ty,
    /// Whether the column is in plane A.
    pub plane_a: bool,
    /// The owning system's Registry id.
    pub owner: String,
}

/// A declared read of a foreign column (`[[reads]]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadDecl {
    /// The coupling's `neutral` value (NEW-4, minor 5), typed by the column.
    pub neutral: Option<Value>,
    /// An identity read (`use = "identity"`, EVO-001 R12): registration refuses a world without
    /// its owner, so it needs no `neutral` (the Architect's identity-read ruling).
    pub identity: bool,
}

/// A Registry parameter of a system (§4.7), with its bounds already in the parameter's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParamDecl {
    /// `Ty::Int` when both bounds are written as integers, else `Ty::Q16`.
    pub int: bool,
    /// The lower bound: the integer itself for an `int` parameter, else its `q16_16` bits rounded
    /// half to even (05.5 §3.2), saturated to `i128`.
    pub lo: i128,
    /// The upper bound, as `lo`.
    pub hi: i128,
}

impl ParamDecl {
    /// The parameter's type.
    #[must_use]
    pub fn ty(&self) -> Ty {
        if self.int { Ty::Int } else { Ty::Q16 }
    }
}

/// A declaring system's declarations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SystemDecl {
    /// Its declared reads of other systems' columns, by column name.
    pub reads: BTreeMap<String, ReadDecl>,
    /// Its declared sends (`[[commands]]`): command name to declared target.
    pub sends: BTreeMap<String, String>,
    /// Its `[[rng]]` purposes.
    pub rng: BTreeSet<String>,
    /// Its Registry parameters.
    pub params: BTreeMap<String, ParamDecl>,
}

/// A command signature of `[commands]` (§6, §7.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandDef {
    /// The command's name.
    pub name: String,
    /// The owning system.
    pub owner: String,
    /// The owner's name of the queue-key field (`victim`, `holder`, …); an action names it
    /// `person` (§6).
    pub key_field: String,
    /// The parameters in payload order, with their language types.
    pub params: Vec<(String, Ty)>,
}

/// The columns the prelude's relations read, resolved once (§5.3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PreludeColumns {
    /// `life_course.mother`.
    pub lc_mother: Option<ColumnId>,
    /// `life_course.father`.
    pub lc_father: Option<ColumnId>,
    /// `life_course.birth_tick`.
    pub lc_birth_tick: Option<ColumnId>,
    /// `person.alive`.
    pub person_alive: Option<ColumnId>,
    /// `tenure.title`.
    pub tenure_title: Option<ColumnId>,
    /// `tenure.holder`.
    pub tenure_holder: Option<ColumnId>,
    /// `title.holder`.
    pub title_holder: Option<ColumnId>,
    /// `title.liege`.
    pub title_liege: Option<ColumnId>,
    /// `office.title`.
    pub office_title: Option<ColumnId>,
    /// `office.holder`.
    pub office_holder: Option<ColumnId>,
    /// `claim.title`.
    pub claim_title: Option<ColumnId>,
    /// `claim.claimant`.
    pub claim_claimant: Option<ColumnId>,
    /// `vow.vower`.
    pub vow_vower: Option<ColumnId>,
}

/// The kinds every environment holds.
pub const PRELUDE_KINDS: [&str; 8] = [
    "person",
    "life_course",
    "tenure",
    "title",
    "office",
    "claim",
    "vow",
    "artifact",
];

/// Why an environment cannot be built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvError(pub String);

impl std::fmt::Display for EnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EnvError {}

/// The environment of §7.4.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Env {
    kinds: Vec<String>,
    kind_index: BTreeMap<String, KindId>,
    enums: Vec<EnumDef>,
    enum_index: BTreeMap<String, EnumId>,
    columns: Vec<ColumnDef>,
    column_index: BTreeMap<String, ColumnId>,
    commands: Vec<CommandDef>,
    command_index: BTreeMap<String, CommandId>,
    /// The declaring systems, by Registry id.
    pub systems: BTreeMap<String, SystemDecl>,
    /// The systems the world registers (NEW-4: reads of others take their neutral, sends to
    /// others are dropped).
    pub registered: BTreeSet<String>,
    prelude: PreludeColumns,
}

impl Env {
    /// An environment holding only the entity kinds the prelude and §7.1's bound variables
    /// name (`person`, `life_course`, `tenure`, `title`, `office`, `claim`, `vow`, `artifact`).
    #[must_use]
    pub fn new() -> Self {
        let mut env = Self::default();
        for kind in PRELUDE_KINDS {
            let _ = env.intern_kind(kind);
        }
        env
    }

    /// Interns an entity kind, returning its id (an existing kind keeps its id).
    ///
    /// # Errors
    /// More than `u16::MAX` kinds.
    pub fn intern_kind(&mut self, name: &str) -> Result<KindId, EnvError> {
        if let Some(&id) = self.kind_index.get(name) {
            return Ok(id);
        }
        let id = KindId(
            u16::try_from(self.kinds.len())
                .map_err(|_| EnvError("too many entity kinds".into()))?,
        );
        self.kinds.push(name.to_owned());
        self.kind_index.insert(name.to_owned(), id);
        Ok(id)
    }

    /// Adds an enum with its values and default.
    ///
    /// # Errors
    /// A duplicate enum, a duplicate value, or a default that is not one of the values.
    pub fn add_enum(
        &mut self,
        name: &str,
        values: Vec<String>,
        default: &str,
    ) -> Result<EnumId, EnvError> {
        if self.enum_index.contains_key(name) {
            return Err(EnvError(format!("enum `{name}` is declared twice")));
        }
        let unique: BTreeSet<&String> = values.iter().collect();
        if unique.len() != values.len() {
            return Err(EnvError(format!("enum `{name}` repeats a value")));
        }
        let default = values.iter().position(|v| v == default).ok_or_else(|| {
            EnvError(format!(
                "enum `{name}`: default `{default}` is not one of its values"
            ))
        })?;
        let id =
            EnumId(u16::try_from(self.enums.len()).map_err(|_| EnvError("too many enums".into()))?);
        self.enums.push(EnumDef {
            name: name.to_owned(),
            values,
            default: u32::try_from(default).map_err(|_| EnvError("enum too large".into()))?,
        });
        self.enum_index.insert(name.to_owned(), id);
        Ok(id)
    }

    /// Adds a column.
    ///
    /// # Errors
    /// A duplicate column name.
    pub fn add_column(&mut self, def: ColumnDef) -> Result<ColumnId, EnvError> {
        if self.column_index.contains_key(&def.name) {
            return Err(EnvError(format!("column `{}` is declared twice", def.name)));
        }
        let id = ColumnId(
            u32::try_from(self.columns.len()).map_err(|_| EnvError("too many columns".into()))?,
        );
        let slot = match def.name.as_str() {
            "life_course.mother" => Some(&mut self.prelude.lc_mother),
            "life_course.father" => Some(&mut self.prelude.lc_father),
            "life_course.birth_tick" => Some(&mut self.prelude.lc_birth_tick),
            "person.alive" => Some(&mut self.prelude.person_alive),
            "tenure.title" => Some(&mut self.prelude.tenure_title),
            "tenure.holder" => Some(&mut self.prelude.tenure_holder),
            "title.holder" => Some(&mut self.prelude.title_holder),
            "title.liege" => Some(&mut self.prelude.title_liege),
            "office.title" => Some(&mut self.prelude.office_title),
            "office.holder" => Some(&mut self.prelude.office_holder),
            "claim.title" => Some(&mut self.prelude.claim_title),
            "claim.claimant" => Some(&mut self.prelude.claim_claimant),
            "vow.vower" => Some(&mut self.prelude.vow_vower),
            _ => None,
        };
        if let Some(slot) = slot {
            *slot = Some(id);
        }
        self.column_index.insert(def.name.clone(), id);
        self.columns.push(def);
        Ok(id)
    }

    /// Adds a command signature.
    ///
    /// # Errors
    /// A duplicate command name.
    pub fn add_command(&mut self, def: CommandDef) -> Result<CommandId, EnvError> {
        if self.command_index.contains_key(&def.name) {
            return Err(EnvError(format!(
                "command `{}` is declared twice",
                def.name
            )));
        }
        let id = CommandId(
            u32::try_from(self.commands.len()).map_err(|_| EnvError("too many commands".into()))?,
        );
        self.command_index.insert(def.name.clone(), id);
        self.commands.push(def);
        Ok(id)
    }

    /// The kind of a name, if declared.
    #[must_use]
    pub fn kind(&self, name: &str) -> Option<KindId> {
        self.kind_index.get(name).copied()
    }

    /// A kind's name.
    #[must_use]
    pub fn kind_name(&self, kind: KindId) -> &str {
        self.kinds
            .get(usize::from(kind.0))
            .map_or("?", String::as_str)
    }

    /// Every kind, in interning order.
    #[must_use]
    pub fn kinds(&self) -> &[String] {
        &self.kinds
    }

    /// An enum by name.
    #[must_use]
    pub fn enum_id(&self, name: &str) -> Option<EnumId> {
        self.enum_index.get(name).copied()
    }

    /// An enum's definition.
    #[must_use]
    pub fn enum_def(&self, id: EnumId) -> Option<&EnumDef> {
        self.enums.get(usize::from(id.0))
    }

    /// Every enum, in declaration order.
    #[must_use]
    pub fn enums(&self) -> &[EnumDef] {
        &self.enums
    }

    /// A column by name.
    #[must_use]
    pub fn column_id(&self, name: &str) -> Option<ColumnId> {
        self.column_index.get(name).copied()
    }

    /// A column's definition.
    #[must_use]
    pub fn column(&self, id: ColumnId) -> Option<&ColumnDef> {
        usize::try_from(id.0).ok().and_then(|i| self.columns.get(i))
    }

    /// Every column, in declaration order.
    #[must_use]
    pub fn columns(&self) -> &[ColumnDef] {
        &self.columns
    }

    /// A command by name.
    #[must_use]
    pub fn command_id(&self, name: &str) -> Option<CommandId> {
        self.command_index.get(name).copied()
    }

    /// A command's signature.
    #[must_use]
    pub fn command(&self, id: CommandId) -> Option<&CommandDef> {
        usize::try_from(id.0)
            .ok()
            .and_then(|i| self.commands.get(i))
    }

    /// Every command, in declaration order.
    #[must_use]
    pub fn commands(&self) -> &[CommandDef] {
        &self.commands
    }

    /// The prelude's resolved columns.
    #[must_use]
    pub fn prelude(&self) -> &PreludeColumns {
        &self.prelude
    }

    /// The zero of a scalar type: the value a missing read yields (§4.5); an enum yields its
    /// named default.
    #[must_use]
    pub fn zero_of(&self, ty: &Ty) -> Value {
        match ty {
            Ty::Bool => Value::Bool(false),
            Ty::Q8 => Value::Q8(rh_core::fixed::Q8_8::ZERO),
            Ty::Q16 => Value::Q16(rh_core::fixed::Q16_16::ZERO),
            Ty::Enum(e) => Value::Enum(*e, self.enum_def(*e).map_or(0, |d| d.default)),
            Ty::Ent(k) => Value::Ent(*k, 0),
            _ => Value::Int(0),
        }
    }

    /// Renders a type as the reference spells it (`ent:person`, `enum:sex`, …), for messages.
    #[must_use]
    pub fn type_name(&self, ty: &Ty) -> String {
        match ty {
            Ty::Bool => "bool".into(),
            Ty::Int => "int".into(),
            Ty::Q8 => "q8_8".into(),
            Ty::Q16 => "q16_16".into(),
            Ty::Enum(e) => format!(
                "enum:{}",
                self.enum_def(*e).map_or("?", |d| d.name.as_str())
            ),
            Ty::Ent(k) => format!("ent:{}", self.kind_name(*k)),
            Ty::Event => "event".into(),
            Ty::LitInt => "lit_int".into(),
            Ty::LitDec => "lit_dec".into(),
            Ty::LitNone => "lit_none".into(),
            Ty::Rel(k) => format!("rel:{}", self.kind_name(*k)),
            Ty::Tuple(items) => {
                let parts: Vec<String> = items.iter().map(|t| self.type_name(t)).collect();
                format!("tuple({})", parts.join(","))
            }
            Ty::List(item) => format!("list({})", self.type_name(item)),
        }
    }
}
