//! Root signatures of our own helper shaders when the system's HLSL compiler
//! leaves them out (Wine/Proton).
//!
//! Every helper shader of the runtime (the indirect-argument clamp, the
//! interface plane, the light estimate, the copies' compare on the card and
//! the others) states its root signature in its HLSL source, as
//! `#define RS "..."` and `[RootSignature(RS)]`, and the root signature is
//! made from the compiled code (`CreateRootSignature(0, code)`). Microsoft's
//! compiler puts it there as the code's `RTS0` part. Under Wine, `D3DCompile`
//! is Wine's own `d3dcompiler_47`, which is vkd3d-shader's HLSL compiler:
//! that ignores the attribute ("Unrecognized attribute", vkd3d
//! `libs/vkd3d-shader/hlsl.y`), so the code has no `RTS0` part, the root
//! signature is refused, and the helper is not made. Without the clamp every
//! indirect draw and dispatch was dropped.
//!
//! [`create`] tries the code first, as before. Only under Wine
//! (`victory_core::wine::is_wine`), and only when the code has no `RTS0`
//! part, the `#define RS` string of the shader's own source is read here
//! ([`parse`]), turned into a version 1.1 description with the flags HLSL
//! gives by default, serialised and made. On Windows nothing changes.
//!
//! The grammar read is the part of HLSL's root signature language the
//! runtime's shaders use: `RootFlags`, `RootConstants`, root `CBV`, `SRV`
//! and `UAV`, `DescriptorTable` of `CBV`, `SRV`, `UAV` and `Sampler`
//! ranges, and `StaticSampler`, with the usual named arguments. Anything
//! else is refused with a message, and the helper is not made, as before.

/// The `#define RS "..."` string of an HLSL source (the first one).
pub(crate) fn source_string(source: &str) -> Option<&str> {
    let at = source.find("#define RS")?;
    let rest = source[at + "#define RS".len()..].trim_start();
    let rest = rest.strip_prefix('"')?;
    Some(&rest[..rest.find('"')?])
}

/// Whether DXBC or DXIL container `code` has a part named `name`.
pub(crate) fn has_part(code: &[u8], name: &[u8; 4]) -> bool {
    let word = |at: usize| {
        code.get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    if code.get(..4) != Some(b"DXBC") {
        return false;
    }
    let Some(count) = word(28) else { return false };
    (0..count.min(64))
        .any(|i| word(32 + 4 * i).and_then(|at| code.get(at..at + 4)) == Some(&name[..]))
}

/// A shader register: kind letter (`b`, `t`, `u`, `s`) and number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Register {
    pub kind: char,
    pub number: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Cbv,
    Srv,
    Uav,
    Sampler,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Parameter {
    Constants {
        register: u32,
        space: u32,
        count: u32,
        visibility: u32,
    },
    Descriptor {
        kind: Kind,
        register: u32,
        space: u32,
        visibility: u32,
        flags: Option<u32>,
    },
    Table {
        ranges: Vec<Range>,
        visibility: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Range {
    pub kind: Kind,
    pub register: u32,
    pub space: u32,
    /// `u32::MAX`: unbounded.
    pub count: u32,
    /// `u32::MAX`: appended after the range before.
    pub offset: u32,
    pub flags: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sampler {
    pub register: u32,
    pub space: u32,
    pub visibility: u32,
    pub filter: u32,
    pub address: [u32; 3],
    pub mip_lod_bias: f32,
    pub max_anisotropy: u32,
    pub comparison: u32,
    pub border: u32,
    pub min_lod: f32,
    pub max_lod: f32,
}

/// A root signature as its HLSL string says it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RootSignature {
    pub flags: u32,
    pub parameters: Vec<Parameter>,
    pub samplers: Vec<Sampler>,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Number(f64),
    Open,
    Close,
    Comma,
    Equals,
    Bar,
}

fn tokens(text: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\r' | '\n' => i += 1,
            '(' | ')' | ',' | '=' | '|' => {
                out.push(match c {
                    '(' => Token::Open,
                    ')' => Token::Close,
                    ',' => Token::Comma,
                    '=' => Token::Equals,
                    _ => Token::Bar,
                });
                i += 1;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                out.push(Token::Word(chars[start..i].iter().collect()));
            }
            c if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' => {
                let start = i;
                i += 1;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric()
                        || chars[i] == '.'
                        || chars[i] == '-'
                        || chars[i] == '+')
                {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let word = word.trim_end_matches(['f', 'F']);
                let value = match word.strip_prefix("0x").or_else(|| word.strip_prefix("0X")) {
                    Some(hex) => u64::from_str_radix(hex, 16).map(|v| v as f64).ok(),
                    None => word.parse::<f64>().ok(),
                };
                out.push(Token::Number(
                    value.ok_or_else(|| format!("not a number: {word}"))?,
                ));
            }
            other => return Err(format!("unexpected '{other}'")),
        }
    }
    Ok(out)
}

/// A clause `Name(args)`; an argument is a register, `name = value` (a
/// value may be `A | B`), or a nested clause.
#[derive(Debug, Clone)]
enum Item {
    Clause(Clause),
    Register(Register),
    Named(String, Vec<Value>),
}

#[derive(Debug, Clone)]
enum Value {
    Word(String),
    Number(f64),
}

#[derive(Debug, Clone)]
struct Clause {
    name: String,
    items: Vec<Item>,
}

struct Reader {
    tokens: Vec<Token>,
    at: usize,
}

impl Reader {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.at).cloned();
        self.at += 1;
        t
    }

    fn expect(&mut self, want: Token) -> Result<(), String> {
        match self.next() {
            Some(t) if t == want => Ok(()),
            got => Err(format!("expected {want:?}, found {got:?}")),
        }
    }

    fn clauses(&mut self) -> Result<Vec<Clause>, String> {
        let mut out = Vec::new();
        while self.peek().is_some() {
            out.push(self.clause()?);
            match self.peek() {
                Some(Token::Comma) => {
                    self.next();
                }
                None => break,
                got => return Err(format!("expected ',' between clauses, found {got:?}")),
            }
        }
        Ok(out)
    }

    fn clause(&mut self) -> Result<Clause, String> {
        let Some(Token::Word(name)) = self.next() else {
            return Err("expected a clause name".into());
        };
        self.expect(Token::Open)?;
        let mut items = Vec::new();
        if self.peek() == Some(&Token::Close) {
            self.next();
            return Ok(Clause { name, items });
        }
        loop {
            items.push(self.item()?);
            match self.next() {
                Some(Token::Comma) => continue,
                Some(Token::Close) => break,
                got => return Err(format!("in {name}: expected ',' or ')', found {got:?}")),
            }
        }
        Ok(Clause { name, items })
    }

    fn item(&mut self) -> Result<Item, String> {
        match (
            self.tokens.get(self.at).cloned(),
            self.tokens.get(self.at + 1).cloned(),
        ) {
            (Some(Token::Word(_)), Some(Token::Open)) => Ok(Item::Clause(self.clause()?)),
            (Some(Token::Word(name)), Some(Token::Equals)) => {
                self.at += 2;
                let mut values = vec![self.value()?];
                while self.peek() == Some(&Token::Bar) {
                    self.next();
                    values.push(self.value()?);
                }
                Ok(Item::Named(name.to_ascii_lowercase(), values))
            }
            (Some(Token::Word(word)), next)
                if next != Some(Token::Bar) && register(&word).is_some() =>
            {
                self.next();
                register(&word)
                    .map(Item::Register)
                    .ok_or_else(|| format!("not a register: {word}"))
            }
            (Some(Token::Word(_) | Token::Number(_)), _) => {
                // A bare value, or flags `A | B` (`RootFlags(0)`,
                // `RootFlags(ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT | ...)`).
                let mut values = vec![self.value()?];
                while self.peek() == Some(&Token::Bar) {
                    self.next();
                    values.push(self.value()?);
                }
                Ok(Item::Named("value".into(), values))
            }
            (got, _) => Err(format!("unexpected {got:?}")),
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.next() {
            Some(Token::Word(w)) => Ok(Value::Word(w)),
            Some(Token::Number(n)) => Ok(Value::Number(n)),
            got => Err(format!("expected a value, found {got:?}")),
        }
    }
}

fn register(word: &str) -> Option<Register> {
    let mut chars = word.chars();
    let kind = chars.next()?.to_ascii_lowercase();
    let number = chars.as_str().parse().ok()?;
    matches!(kind, 'b' | 't' | 'u' | 's').then_some(Register { kind, number })
}

/// The flags or enumerators of `values`, or'ed, by `names`.
fn flags(values: &[Value], names: &[(&str, u32)]) -> Result<u32, String> {
    values.iter().try_fold(0, |all, v| {
        let one = match v {
            Value::Number(n) => *n as u32,
            Value::Word(w) => names
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(w))
                .map(|(_, bits)| *bits)
                .ok_or_else(|| format!("unknown value {w}"))?,
        };
        Ok(all | one)
    })
}

fn number(values: &[Value]) -> Result<f64, String> {
    match values {
        [Value::Number(n)] => Ok(*n),
        [Value::Word(w)] if w.eq_ignore_ascii_case("unbounded") => Ok(u32::MAX as f64),
        [Value::Word(w)] if w.eq_ignore_ascii_case("DESCRIPTOR_RANGE_OFFSET_APPEND") => {
            Ok(u32::MAX as f64)
        }
        other => Err(format!("expected a number, found {other:?}")),
    }
}

const VISIBILITY: &[(&str, u32)] = &[
    ("SHADER_VISIBILITY_ALL", 0),
    ("SHADER_VISIBILITY_VERTEX", 1),
    ("SHADER_VISIBILITY_HULL", 2),
    ("SHADER_VISIBILITY_DOMAIN", 3),
    ("SHADER_VISIBILITY_GEOMETRY", 4),
    ("SHADER_VISIBILITY_PIXEL", 5),
    ("SHADER_VISIBILITY_AMPLIFICATION", 6),
    ("SHADER_VISIBILITY_MESH", 7),
];

const ROOT_FLAGS: &[(&str, u32)] = &[
    ("ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT", 0x1),
    ("DENY_VERTEX_SHADER_ROOT_ACCESS", 0x2),
    ("DENY_HULL_SHADER_ROOT_ACCESS", 0x4),
    ("DENY_DOMAIN_SHADER_ROOT_ACCESS", 0x8),
    ("DENY_GEOMETRY_SHADER_ROOT_ACCESS", 0x10),
    ("DENY_PIXEL_SHADER_ROOT_ACCESS", 0x20),
    ("ALLOW_STREAM_OUTPUT", 0x40),
    ("LOCAL_ROOT_SIGNATURE", 0x80),
    ("DENY_AMPLIFICATION_SHADER_ROOT_ACCESS", 0x100),
    ("DENY_MESH_SHADER_ROOT_ACCESS", 0x200),
    ("CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED", 0x400),
    ("SAMPLER_HEAP_DIRECTLY_INDEXED", 0x800),
];

/// `D3D12_DESCRIPTOR_RANGE_FLAGS` and `D3D12_ROOT_DESCRIPTOR_FLAGS` (the
/// latter use the same bits).
const RANGE_FLAGS: &[(&str, u32)] = &[
    ("NONE", 0),
    ("DESCRIPTORS_VOLATILE", 0x1),
    ("DATA_VOLATILE", 0x2),
    ("DATA_STATIC_WHILE_SET_AT_EXECUTE", 0x4),
    ("DATA_STATIC", 0x8),
    ("DESCRIPTORS_STATIC_KEEPING_BUFFER_BOUNDS_CHECKS", 0x10000),
];

const FILTERS: &[(&str, u32)] = &[
    ("FILTER_MIN_MAG_MIP_POINT", 0x0),
    ("FILTER_MIN_MAG_POINT_MIP_LINEAR", 0x1),
    ("FILTER_MIN_POINT_MAG_LINEAR_MIP_POINT", 0x4),
    ("FILTER_MIN_POINT_MAG_MIP_LINEAR", 0x5),
    ("FILTER_MIN_LINEAR_MAG_MIP_POINT", 0x10),
    ("FILTER_MIN_LINEAR_MAG_POINT_MIP_LINEAR", 0x11),
    ("FILTER_MIN_MAG_LINEAR_MIP_POINT", 0x14),
    ("FILTER_MIN_MAG_MIP_LINEAR", 0x15),
    ("FILTER_ANISOTROPIC", 0x55),
    ("FILTER_COMPARISON_MIN_MAG_MIP_POINT", 0x80),
    ("FILTER_COMPARISON_MIN_MAG_MIP_LINEAR", 0x95),
    ("FILTER_COMPARISON_ANISOTROPIC", 0xd5),
];

const ADDRESS: &[(&str, u32)] = &[
    ("TEXTURE_ADDRESS_WRAP", 1),
    ("TEXTURE_ADDRESS_MIRROR", 2),
    ("TEXTURE_ADDRESS_CLAMP", 3),
    ("TEXTURE_ADDRESS_BORDER", 4),
    ("TEXTURE_ADDRESS_MIRROR_ONCE", 5),
];

const COMPARISON: &[(&str, u32)] = &[
    ("COMPARISON_NEVER", 1),
    ("COMPARISON_LESS", 2),
    ("COMPARISON_EQUAL", 3),
    ("COMPARISON_LESS_EQUAL", 4),
    ("COMPARISON_GREATER", 5),
    ("COMPARISON_NOT_EQUAL", 6),
    ("COMPARISON_GREATER_EQUAL", 7),
    ("COMPARISON_ALWAYS", 8),
];

const BORDER: &[(&str, u32)] = &[
    ("STATIC_BORDER_COLOR_TRANSPARENT_BLACK", 0),
    ("STATIC_BORDER_COLOR_OPAQUE_BLACK", 1),
    ("STATIC_BORDER_COLOR_OPAQUE_WHITE", 2),
];

fn kind_of(name: &str) -> Option<Kind> {
    match name.to_ascii_uppercase().as_str() {
        "CBV" => Some(Kind::Cbv),
        "SRV" => Some(Kind::Srv),
        "UAV" => Some(Kind::Uav),
        "SAMPLER" => Some(Kind::Sampler),
        _ => None,
    }
}

fn letter(kind: Kind) -> char {
    match kind {
        Kind::Cbv => 'b',
        Kind::Srv => 't',
        Kind::Uav => 'u',
        Kind::Sampler => 's',
    }
}

/// The register of a clause, which must be of `kind`'s letter.
fn the_register(clause: &Clause, kind: char) -> Result<u32, String> {
    clause
        .items
        .iter()
        .find_map(|i| match i {
            Item::Register(r) => Some(*r),
            _ => None,
        })
        .filter(|r| r.kind == kind)
        .map(|r| r.number)
        .ok_or_else(|| format!("{}: no {kind} register", clause.name))
}

fn named<'a>(clause: &'a Clause, name: &str) -> Option<&'a [Value]> {
    clause.items.iter().find_map(|i| match i {
        Item::Named(n, v) if n == name => Some(&v[..]),
        _ => None,
    })
}

fn visibility(clause: &Clause) -> Result<u32, String> {
    named(clause, "visibility").map_or(Ok(0), |v| flags(v, VISIBILITY))
}

fn space(clause: &Clause) -> Result<u32, String> {
    named(clause, "space").map_or(Ok(0), |v| number(v).map(|n| n as u32))
}

fn known(clause: &Clause, names: &[&str]) -> Result<(), String> {
    for item in &clause.items {
        match item {
            Item::Named(n, _) if !names.contains(&n.as_str()) => {
                return Err(format!("{}: unknown argument {n}", clause.name));
            }
            Item::Clause(c) if clause.name != "DescriptorTable" => {
                return Err(format!("{}: unexpected {}", clause.name, c.name));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Reads an HLSL root signature string.
pub(crate) fn parse(text: &str) -> Result<RootSignature, String> {
    let mut reader = Reader {
        tokens: tokens(text)?,
        at: 0,
    };
    let mut out = RootSignature::default();
    for clause in reader.clauses()? {
        match clause.name.as_str() {
            "RootFlags" => {
                known(&clause, &["value"])?;
                out.flags |= named(&clause, "value").map_or(Ok(0), |v| flags(v, ROOT_FLAGS))?;
            }
            "RootConstants" => {
                known(&clause, &["num32bitconstants", "space", "visibility"])?;
                let count = named(&clause, "num32bitconstants")
                    .ok_or("RootConstants: no num32BitConstants")?;
                out.parameters.push(Parameter::Constants {
                    register: the_register(&clause, 'b')?,
                    space: space(&clause)?,
                    count: number(count)? as u32,
                    visibility: visibility(&clause)?,
                });
            }
            "CBV" | "SRV" | "UAV" => {
                known(&clause, &["space", "visibility", "flags"])?;
                let kind = kind_of(&clause.name).ok_or("unreachable")?;
                out.parameters.push(Parameter::Descriptor {
                    kind,
                    register: the_register(&clause, letter(kind))?,
                    space: space(&clause)?,
                    visibility: visibility(&clause)?,
                    flags: named(&clause, "flags")
                        .map(|v| flags(v, RANGE_FLAGS))
                        .transpose()?,
                });
            }
            "DescriptorTable" => {
                known(&clause, &["visibility"])?;
                let mut ranges = Vec::new();
                for item in &clause.items {
                    let Item::Clause(range) = item else { continue };
                    let kind = kind_of(&range.name)
                        .ok_or_else(|| format!("DescriptorTable: unknown range {}", range.name))?;
                    known(range, &["numdescriptors", "space", "offset", "flags"])?;
                    ranges.push(Range {
                        kind,
                        register: the_register(range, letter(kind))?,
                        space: space(range)?,
                        count: named(range, "numdescriptors").map_or(Ok(1.0), number)? as u32,
                        offset: named(range, "offset").map_or(Ok(u32::MAX as f64), number)? as u32,
                        flags: named(range, "flags")
                            .map(|v| flags(v, RANGE_FLAGS))
                            .transpose()?,
                    });
                }
                if ranges.is_empty() {
                    return Err("DescriptorTable without ranges".into());
                }
                out.parameters.push(Parameter::Table {
                    ranges,
                    visibility: visibility(&clause)?,
                });
            }
            "StaticSampler" => {
                known(
                    &clause,
                    &[
                        "filter",
                        "addressu",
                        "addressv",
                        "addressw",
                        "miplodbias",
                        "maxanisotropy",
                        "comparisonfunc",
                        "bordercolor",
                        "minlod",
                        "maxlod",
                        "space",
                        "visibility",
                    ],
                )?;
                let pick = |name: &str, names: &[(&str, u32)], default: u32| {
                    named(&clause, name).map_or(Ok(default), |v| flags(v, names))
                };
                let real = |name: &str, default: f32| {
                    named(&clause, name).map_or(Ok(default), |v| number(v).map(|n| n as f32))
                };
                out.samplers.push(Sampler {
                    register: the_register(&clause, 's')?,
                    space: space(&clause)?,
                    visibility: visibility(&clause)?,
                    filter: pick("filter", FILTERS, 0x55)?,
                    address: [
                        pick("addressu", ADDRESS, 1)?,
                        pick("addressv", ADDRESS, 1)?,
                        pick("addressw", ADDRESS, 1)?,
                    ],
                    mip_lod_bias: real("miplodbias", 0.0)?,
                    max_anisotropy: named(&clause, "maxanisotropy").map_or(Ok(16.0), number)?
                        as u32,
                    comparison: pick("comparisonfunc", COMPARISON, 4)?,
                    border: pick("bordercolor", BORDER, 2)?,
                    min_lod: real("minlod", 0.0)?,
                    max_lod: real("maxlod", f32::MAX)?,
                });
            }
            other => return Err(format!("unknown clause {other}")),
        }
    }
    Ok(out)
}

/// The data flags HLSL's version 1.1 root signatures get when a range or
/// root descriptor names none: constant buffers and shader resources static
/// while set at execute, unordered access volatile, samplers none.
fn default_flags(kind: Kind) -> u32 {
    match kind {
        Kind::Cbv | Kind::Srv => 0x4,
        Kind::Uav => 0x2,
        Kind::Sampler => 0,
    }
}

#[cfg(windows)]
pub(crate) use host::create;

#[cfg(windows)]
mod host {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use windows::Win32::Graphics::Direct3D::ID3DBlob;
    use windows::Win32::Graphics::Direct3D12::*;

    /// Root signatures made from a source's string so far ([`create`]).
    static MADE: AtomicU64 = AtomicU64::new(0);

    /// The root signature of a helper shader compiled from `source` into
    /// `code`: the one inside the code; under Wine, where the compiler left
    /// it out, the one `source`'s `#define RS` string says.
    pub(crate) fn create(
        device: &ID3D12Device,
        code: &[u8],
        source: &str,
    ) -> windows::core::Result<ID3D12RootSignature> {
        // SAFETY: the bytes of a compiled shader, alive through the call.
        let inside = unsafe { device.CreateRootSignature::<ID3D12RootSignature>(0, code) };
        let error = match inside {
            Ok(root) => return Ok(root),
            Err(e) => e,
        };
        if !victory_core::wine::is_wine() || has_part(code, b"RTS0") {
            return Err(error);
        }
        let Some(text) = source_string(source) else {
            return Err(error);
        };
        let parsed = match parse(text) {
            Ok(parsed) => parsed,
            Err(why) => {
                tracing::warn!(%why, text, "a helper shader's root signature could not be read from its source");
                return Err(error);
            }
        };
        let root = from_parsed(device, &parsed)?;
        if MADE.fetch_add(1, Ordering::Relaxed) == 0 {
            tracing::info!(
                "the system's HLSL compiler left helper shaders' root signatures out (Wine's builtin d3dcompiler_47): \
                 they are made from their sources' strings instead"
            );
        }
        tracing::debug!(text, "root signature made from the source's string");
        Ok(root)
    }

    /// Serialises `rs` as a version 1.1 root signature and makes it.
    pub(crate) fn from_parsed(
        device: &ID3D12Device,
        rs: &RootSignature,
    ) -> windows::core::Result<ID3D12RootSignature> {
        let visibility = |v: u32| D3D12_SHADER_VISIBILITY(v as i32);
        let range_type = |k: Kind| match k {
            Kind::Cbv => D3D12_DESCRIPTOR_RANGE_TYPE_CBV,
            Kind::Srv => D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
            Kind::Uav => D3D12_DESCRIPTOR_RANGE_TYPE_UAV,
            Kind::Sampler => D3D12_DESCRIPTOR_RANGE_TYPE_SAMPLER,
        };
        // Every table's ranges, kept apart so the pointers stay put.
        let tables: Vec<Vec<D3D12_DESCRIPTOR_RANGE1>> = rs
            .parameters
            .iter()
            .map(|p| match p {
                Parameter::Table { ranges, .. } => ranges
                    .iter()
                    .map(|r| D3D12_DESCRIPTOR_RANGE1 {
                        RangeType: range_type(r.kind),
                        NumDescriptors: r.count,
                        BaseShaderRegister: r.register,
                        RegisterSpace: r.space,
                        Flags: D3D12_DESCRIPTOR_RANGE_FLAGS(
                            r.flags.unwrap_or(default_flags(r.kind)) as i32,
                        ),
                        OffsetInDescriptorsFromTableStart: r.offset,
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .collect();
        let parameters: Vec<D3D12_ROOT_PARAMETER1> = rs
            .parameters
            .iter()
            .zip(&tables)
            .map(|(p, table)| match p {
                Parameter::Constants {
                    register,
                    space,
                    count,
                    visibility: v,
                } => D3D12_ROOT_PARAMETER1 {
                    ParameterType: D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS,
                    Anonymous: D3D12_ROOT_PARAMETER1_0 {
                        Constants: D3D12_ROOT_CONSTANTS {
                            ShaderRegister: *register,
                            RegisterSpace: *space,
                            Num32BitValues: *count,
                        },
                    },
                    ShaderVisibility: visibility(*v),
                },
                Parameter::Descriptor {
                    kind,
                    register,
                    space,
                    visibility: v,
                    flags,
                } => D3D12_ROOT_PARAMETER1 {
                    ParameterType: match kind {
                        Kind::Cbv => D3D12_ROOT_PARAMETER_TYPE_CBV,
                        Kind::Srv => D3D12_ROOT_PARAMETER_TYPE_SRV,
                        _ => D3D12_ROOT_PARAMETER_TYPE_UAV,
                    },
                    Anonymous: D3D12_ROOT_PARAMETER1_0 {
                        Descriptor: D3D12_ROOT_DESCRIPTOR1 {
                            ShaderRegister: *register,
                            RegisterSpace: *space,
                            Flags: D3D12_ROOT_DESCRIPTOR_FLAGS(
                                flags.unwrap_or(default_flags(*kind)) as i32,
                            ),
                        },
                    },
                    ShaderVisibility: visibility(*v),
                },
                Parameter::Table { visibility: v, .. } => D3D12_ROOT_PARAMETER1 {
                    ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
                    Anonymous: D3D12_ROOT_PARAMETER1_0 {
                        DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE1 {
                            NumDescriptorRanges: table.len() as u32,
                            pDescriptorRanges: table.as_ptr(),
                        },
                    },
                    ShaderVisibility: visibility(*v),
                },
            })
            .collect();
        let samplers: Vec<D3D12_STATIC_SAMPLER_DESC> = rs
            .samplers
            .iter()
            .map(|s| D3D12_STATIC_SAMPLER_DESC {
                Filter: D3D12_FILTER(s.filter as i32),
                AddressU: D3D12_TEXTURE_ADDRESS_MODE(s.address[0] as i32),
                AddressV: D3D12_TEXTURE_ADDRESS_MODE(s.address[1] as i32),
                AddressW: D3D12_TEXTURE_ADDRESS_MODE(s.address[2] as i32),
                MipLODBias: s.mip_lod_bias,
                MaxAnisotropy: s.max_anisotropy,
                ComparisonFunc: D3D12_COMPARISON_FUNC(s.comparison as i32),
                BorderColor: D3D12_STATIC_BORDER_COLOR(s.border as i32),
                MinLOD: s.min_lod,
                MaxLOD: s.max_lod,
                ShaderRegister: s.register,
                RegisterSpace: s.space,
                ShaderVisibility: visibility(s.visibility),
            })
            .collect();
        let desc = D3D12_VERSIONED_ROOT_SIGNATURE_DESC {
            Version: D3D_ROOT_SIGNATURE_VERSION_1_1,
            Anonymous: D3D12_VERSIONED_ROOT_SIGNATURE_DESC_0 {
                Desc_1_1: D3D12_ROOT_SIGNATURE_DESC1 {
                    NumParameters: parameters.len() as u32,
                    pParameters: parameters.as_ptr(),
                    NumStaticSamplers: samplers.len() as u32,
                    pStaticSamplers: samplers.as_ptr(),
                    Flags: D3D12_ROOT_SIGNATURE_FLAGS(rs.flags as i32),
                },
            },
        };
        let mut blob: Option<ID3DBlob> = None;
        let mut error: Option<ID3DBlob> = None;
        // SAFETY: the description and everything it points at live through
        // the calls; the blob's own bytes are read while it lives.
        unsafe {
            if let Err(e) = D3D12SerializeVersionedRootSignature(&desc, &mut blob, Some(&mut error))
            {
                if let Some(error) = error {
                    let text = std::slice::from_raw_parts(
                        error.GetBufferPointer().cast::<u8>(),
                        error.GetBufferSize(),
                    );
                    tracing::warn!("helper root signature: {}", String::from_utf8_lossy(text));
                }
                return Err(e);
            }
            let blob = blob.ok_or_else(windows::core::Error::empty)?;
            let bytes = std::slice::from_raw_parts(
                blob.GetBufferPointer().cast::<u8>(),
                blob.GetBufferSize(),
            );
            device.CreateRootSignature(0, bytes)
        }
    }
}

/// Every helper shader the runtime compiles with the system's HLSL compiler:
/// name, source, entry point and target (`victory hlsl-check`).
#[cfg(windows)]
pub(crate) fn helpers() -> Vec<(String, String, &'static str, &'static str)> {
    let mut all: Vec<(String, String, &'static str, &'static str)> = vec![
        (
            "indirect clamp".into(),
            crate::indirect::clamp_source(crate::indirect::SOURCE, false),
            "main",
            "cs_5_1",
        ),
        (
            "indirect clamp, batched".into(),
            crate::indirect::clamp_source(crate::indirect::batch::SOURCE, false),
            "main",
            "cs_5_1",
        ),
        (
            "interface plane (vs)".into(),
            crate::compose::SOURCE.into(),
            "vs",
            "vs_5_1",
        ),
        (
            "interface plane (ps)".into(),
            crate::compose::SOURCE.into(),
            "ps",
            "ps_5_1",
        ),
        (
            "card compare".into(),
            crate::host::CARD_COMPARE.into(),
            "main",
            "cs_5_1",
        ),
        (
            "depth plane corners".into(),
            crate::host::PLANE_CORNERS.into(),
            "corner",
            "vs_5_1",
        ),
        (
            "depth plane steps".into(),
            crate::host::PLANE_STEPS.into(),
            "plane",
            "ps_5_1",
        ),
        (
            "depth plane floats".into(),
            crate::host::PLANE_FLOATS.into(),
            "plane",
            "ps_5_1",
        ),
        (
            "card awake".into(),
            crate::host::awake::COUNT.into(),
            "main",
            "cs_5_1",
        ),
        (
            "scatter".into(),
            crate::host::send::SCATTER.into(),
            "main",
            "cs_5_1",
        ),
        (
            "upload check".into(),
            crate::host::upcheck::SOURCE.into(),
            "main",
            "cs_5_1",
        ),
    ];
    for (part, entry) in [
        ("KEEP", "keep"),
        ("KEEP", "mean"),
        ("PLACE", "place"),
        ("GATHER", "gather"),
        ("RAISE", "raise"),
        ("AIR", "clear_air"),
        ("REFLECT", "reflect_light"),
    ] {
        all.push((
            format!("light estimate {entry}"),
            format!("#define {part}\n{}", crate::ambient::SOURCE),
            entry,
            "cs_5_1",
        ));
    }
    all
}

/// `victory hlsl-check`: compiles every helper shader with the system's
/// compiler and says, for each, whether it compiled, whether the compiler put
/// its root signature in, and whether the source's string reads (the Wine
/// fallback of [`create`]); with a host device, whether the root signature is
/// made. CPU only but for that last column.
#[cfg(windows)]
pub fn check(device: bool) -> Vec<String> {
    use windows::core::PCSTR;
    let host = if device { crate::host::host() } else { None };
    let mut lines = vec![format!(
        "{:<28} {:<16} {:<10} {:<14} {}",
        "helper shader",
        "compiles",
        "RTS0 part",
        "string reads",
        if host.is_some() { "root signature" } else { "" }
    )];
    let mut failed = 0;
    for (name, source, entry, target) in helpers() {
        let entry_c = std::ffi::CString::new(entry).unwrap_or_default();
        let target_c = std::ffi::CString::new(target).unwrap_or_default();
        let compiled = crate::host::compile(
            &source,
            PCSTR(entry_c.as_ptr().cast()),
            PCSTR(target_c.as_ptr().cast()),
        );
        let string = source_string(&source).map(parse);
        let reads = match &string {
            Some(Ok(_)) => "yes".to_owned(),
            Some(Err(why)) => format!("NO ({why})"),
            // (A vertex shader made with its pixel shader's root signature.)
            None => "- (its pair's)".to_owned(),
        };
        let line = match compiled {
            Ok(code) => {
                let part = has_part(&code, b"RTS0");
                let made = host.map(|h| match create(&h.device, &code, &source) {
                    Ok(_) => "made".to_owned(),
                    Err(e) => format!("REFUSED ({e})"),
                });
                format!(
                    "{name:<28} {:<16} {:<10} {reads:<14} {}",
                    "yes",
                    if part { "yes" } else { "no" },
                    made.unwrap_or_default()
                )
            }
            Err(e) => {
                failed += 1;
                format!(
                    "{name:<28} {:<16} {:<10} {reads:<14}",
                    format!("NO ({:#x})", e.code().0 as u32),
                    "-"
                )
            }
        };
        lines.push(line);
    }
    if failed > 0 {
        lines.push(String::new());
        lines.push(format!(
            "{failed} helper shaders do not compile with this system's HLSL compiler{}",
            if victory_core::wine::is_wine() {
                " (Wine's builtin d3dcompiler_47). Microsoft's d3dcompiler_47.dll in the prefix fixes it: \
                 `protontricks <appid> d3dcompiler_47`, or `WINEPREFIX=<prefix>/pfx winetricks d3dcompiler_47` \
                 (docs/linux-proton.md)."
            } else {
                "."
            }
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every root signature string the runtime's helper shaders use, as in
    /// their sources.
    const USED: &[&str] = &[
        "UAV(u0)",
        "RootFlags(0)",
        "DescriptorTable(SRV(t0))",
        "DescriptorTable(SRV(t0), visibility = SHADER_VISIBILITY_PIXEL)",
        "DescriptorTable(SRV(t0, numDescriptors = 2)), UAV(u0), StaticSampler(s0, filter = FILTER_MIN_MAG_MIP_POINT)",
        "RootConstants(num32BitConstants=6, b0), SRV(t0), SRV(t1), UAV(u0), UAV(u1)",
        "RootConstants(num32BitConstants=2, b0), SRV(t0), UAV(u0)",
        "RootConstants(num32BitConstants = 1, b0)",
        "RootConstants(num32BitConstants=5, b0), SRV(t0), SRV(t1), UAV(u0)",
        "SRV(t0), SRV(t1), SRV(t2), SRV(t3), SRV(t4), UAV(u0), UAV(u1)",
        "RootConstants(num32BitConstants=6, b0), SRV(t0), UAV(u0), UAV(u1)",
        "RootConstants(num32BitConstants=12, b0), DescriptorTable(SRV(t0)), DescriptorTable(UAV(u0)), UAV(u1), UAV(u2)",
        "RootConstants(num32BitConstants=2, b0), SRV(t0), UAV(u0), UAV(u1), UAV(u2)",
        "DescriptorTable(SRV(t0,numDescriptors=64,flags=DESCRIPTORS_VOLATILE|DATA_VOLATILE)), UAV(u0)",
        "RootConstants(num32BitConstants=4,b0), DescriptorTable(SRV(t0,numDescriptors=4),CBV(b1)), UAV(u0), StaticSampler(s0,filter=FILTER_MIN_MAG_MIP_LINEAR,addressU=TEXTURE_ADDRESS_WRAP,addressV=TEXTURE_ADDRESS_WRAP)",
        "DescriptorTable(SRV(t0, numDescriptors=2), visibility=SHADER_VISIBILITY_PIXEL), StaticSampler(s0, filter=FILTER_MIN_MAG_MIP_LINEAR, addressU=TEXTURE_ADDRESS_CLAMP, addressV=TEXTURE_ADDRESS_CLAMP)",
        "RootConstants(num32BitConstants=2,b0), DescriptorTable(SRV(t0,numDescriptors=4,flags=DESCRIPTORS_VOLATILE)), UAV(u0), StaticSampler(s0,filter=FILTER_MIN_MAG_MIP_POINT,addressU=TEXTURE_ADDRESS_WRAP,addressV=TEXTURE_ADDRESS_WRAP,addressW=TEXTURE_ADDRESS_WRAP)",
        "RootConstants(num32BitConstants=1,b0), DescriptorTable(SRV(t0,numDescriptors=2,flags=DESCRIPTORS_VOLATILE)), UAV(u0)",
        "RootConstants(num32BitConstants=4,b0), DescriptorTable(SRV(t0,numDescriptors=8)), UAV(u0), DescriptorTable(CBV(b1,numDescriptors=8))",
    ];

    #[test]
    fn every_string_the_helpers_use_reads() {
        for text in USED {
            parse(text).unwrap_or_else(|why| panic!("{text}: {why}"));
        }
    }

    /// The strings in the sources themselves, so a new helper with a clause
    /// this reader does not know fails here, not on a player's machine.
    #[test]
    fn every_string_in_the_sources_reads() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut seen = 0;
        for entry in walk(&dir) {
            let text = std::fs::read_to_string(&entry).unwrap_or_default();
            let mut rest = &text[..];
            while let Some(at) = rest.find("#define RS ") {
                rest = &rest[at..];
                // Strings inside Rust string literals may have their quotes
                // escaped (`\"`).
                let unescaped = rest.replace("\\\"", "\"");
                if let Some(rs) = source_string(&unescaped) {
                    parse(rs).unwrap_or_else(|why| panic!("{}: {rs}: {why}", entry.display()));
                    seen += 1;
                }
                rest = &rest["#define RS ".len()..];
            }
        }
        assert!(seen >= 20, "only {seen} root signature strings found");
    }

    fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else if p.extension().is_some_and(|x| x == "rs") && !p.ends_with("rootsig.rs") {
                out.push(p);
            }
        }
        out
    }

    #[test]
    fn a_table_with_two_kinds_and_a_sampler() {
        let rs = parse(USED[14]).unwrap();
        assert_eq!(rs.parameters.len(), 3);
        assert_eq!(
            rs.parameters[0],
            Parameter::Constants {
                register: 0,
                space: 0,
                count: 4,
                visibility: 0
            }
        );
        let Parameter::Table { ranges, visibility } = &rs.parameters[1] else {
            panic!("{:?}", rs.parameters[1])
        };
        assert_eq!(*visibility, 0);
        assert_eq!(ranges.len(), 2);
        assert_eq!(
            (
                ranges[0].kind,
                ranges[0].register,
                ranges[0].count,
                ranges[0].offset
            ),
            (Kind::Srv, 0, 4, u32::MAX)
        );
        assert_eq!(
            (ranges[1].kind, ranges[1].register, ranges[1].count),
            (Kind::Cbv, 1, 1)
        );
        assert_eq!(
            rs.parameters[2],
            Parameter::Descriptor {
                kind: Kind::Uav,
                register: 0,
                space: 0,
                visibility: 0,
                flags: None
            }
        );
        assert_eq!(rs.samplers.len(), 1);
        let s = &rs.samplers[0];
        assert_eq!(
            (
                s.filter,
                s.address,
                s.max_anisotropy,
                s.comparison,
                s.border
            ),
            (0x15, [1, 1, 1], 16, 4, 2)
        );
        assert_eq!(s.max_lod, f32::MAX);
    }

    #[test]
    fn flags_visibility_and_defaults() {
        let rs = parse(USED[13]).unwrap();
        let Parameter::Table { ranges, .. } = &rs.parameters[0] else {
            panic!()
        };
        assert_eq!((ranges[0].count, ranges[0].flags), (64, Some(0x3)));
        let rs = parse(USED[15]).unwrap();
        let Parameter::Table { visibility, .. } = &rs.parameters[0] else {
            panic!()
        };
        assert_eq!(*visibility, 5);
        assert_eq!(rs.samplers[0].address, [3, 3, 1]);
        assert_eq!(parse("RootFlags(0)").unwrap(), RootSignature::default());
        assert_eq!(
            parse("RootFlags(ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT | DENY_PIXEL_SHADER_ROOT_ACCESS)")
                .unwrap()
                .flags,
            0x21
        );
        assert_eq!(
            (
                default_flags(Kind::Srv),
                default_flags(Kind::Uav),
                default_flags(Kind::Sampler)
            ),
            (4, 2, 0)
        );
    }

    #[test]
    fn what_is_not_known_is_refused() {
        assert!(parse("Bogus(u0)").is_err());
        assert!(parse("UAV(t0)").is_err());
        assert!(parse("SRV(t0, colour = 3)").is_err());
        assert!(parse("DescriptorTable()").is_err());
        assert!(parse("StaticSampler(s0, filter = FILTER_SOMETHING)").is_err());
        assert!(parse("UAV(u0").is_err());
    }

    /// Every string, serialised and made on a device (any: the first
    /// adapter or the software one; skipped without one).
    #[cfg(windows)]
    #[test]
    fn every_string_makes_a_root_signature() {
        use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
        use windows::Win32::Graphics::Direct3D12::{D3D12CreateDevice, ID3D12Device};
        let mut device: Option<ID3D12Device> = None;
        // SAFETY: plain device creation.
        if unsafe { D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device) }.is_err() {
            eprintln!("no D3D12 device; skipped");
            return;
        }
        let device = device.expect("a device");
        let refused: Vec<String> = USED
            .iter()
            .filter_map(|text| {
                host::from_parsed(&device, &parse(text).unwrap())
                    .err()
                    .map(|e| format!("{text}: {e}"))
            })
            .collect();
        assert!(
            refused.is_empty(),
            "{} of {} refused:\n{}",
            refused.len(),
            USED.len(),
            refused.join("\n")
        );
    }

    #[test]
    fn the_string_is_found_in_a_source() {
        let source = "\n#define RS \"UAV(u0)\"\nRWByteAddressBuffer b : register(u0);\n[RootSignature(RS)]\n";
        assert_eq!(source_string(source), Some("UAV(u0)"));
        assert_eq!(source_string("no string here"), None);
    }

    #[test]
    fn container_parts_are_found() {
        // A DXBC header naming two parts, ISGN and RTS0.
        let mut code = b"DXBC".to_vec();
        code.extend([0u8; 16]); // checksum
        code.extend(1u32.to_le_bytes()); // version
        code.extend(0u32.to_le_bytes()); // size (not read)
        code.extend(2u32.to_le_bytes()); // parts
        code.extend(40u32.to_le_bytes());
        code.extend(48u32.to_le_bytes());
        code.extend(b"ISGN\0\0\0\0RTS0\0\0\0\0");
        assert!(has_part(&code, b"RTS0") && has_part(&code, b"ISGN"));
        assert!(!has_part(&code, b"SHEX"));
        assert!(!has_part(b"DXIL", b"RTS0"));
    }
}
