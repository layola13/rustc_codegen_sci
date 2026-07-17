use std::fmt;
use std::io::{self, Read, Write};

pub const RPC_MAGIC: [u8; 8] = *b"SCIRPC\0\0";
pub const RPC_VERSION: u16 = 1;
pub const PLAN_VERSION: u16 = 8;
pub const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Endian {
    Little = 1,
    Big = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ScalarType {
    I1 = 0,
    I8 = 1,
    I16 = 2,
    I32 = 3,
    I64 = 4,
    U8 = 5,
    U16 = 6,
    U32 = 7,
    U64 = 8,
    Ptr = 9,
}

impl ScalarType {
    pub fn sa_name(self) -> &'static str {
        match self {
            Self::I1 => "i1",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::Ptr => "ptr",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BinaryOp {
    Add = 1,
    Sub = 2,
    Mul = 3,
    BitXor = 4,
    BitAnd = 5,
    BitOr = 6,
    SDiv = 7,
    UDiv = 8,
    SRem = 9,
    URem = 10,
    Shl = 11,
    LShr = 12,
    AShr = 13,
}

impl BinaryOp {
    pub fn sa_name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "mul",
            Self::BitXor => "xor",
            Self::BitAnd => "and",
            Self::BitOr => "or",
            Self::SDiv => "sdiv",
            Self::UDiv => "udiv",
            Self::SRem => "srem",
            Self::URem => "urem",
            Self::Shl => "shl",
            Self::LShr => "lshr",
            Self::AShr => "ashr",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CompareOp {
    Eq = 1,
    Ne = 2,
    Slt = 3,
    Sle = 4,
    Sgt = 5,
    Sge = 6,
    Ult = 7,
    Ule = 8,
    Ugt = 9,
    Uge = 10,
}

impl CompareOp {
    pub fn sa_name(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Slt => "slt",
            Self::Sle => "sle",
            Self::Sgt => "sgt",
            Self::Sge => "sge",
            Self::Ult => "ult",
            Self::Ule => "ule",
            Self::Ugt => "ugt",
            Self::Uge => "uge",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CastOp {
    Trunc = 1,
    Zext = 2,
    Sext = 3,
    Bitcast = 4,
}

impl CastOp {
    pub fn sa_name(self) -> &'static str {
        match self {
            Self::Trunc => "trunc",
            Self::Zext => "zext",
            Self::Sext => "sext",
            Self::Bitcast => "bitcast",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueRef {
    Local(u32),
    Integer { ty: ScalarType, bits: u64 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operation {
    Copy {
        dst: u32,
        src: ValueRef,
    },
    Binary {
        dst: u32,
        op: BinaryOp,
        lhs: ValueRef,
        rhs: ValueRef,
    },
    Compare {
        dst: u32,
        op: CompareOp,
        lhs: ValueRef,
        rhs: ValueRef,
    },
    Cast {
        dst: u32,
        op: CastOp,
        src: ValueRef,
        ty: ScalarType,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalPlan {
    pub id: u32,
    pub ty: ScalarType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CallingConventionPlan {
    C,
    Rust,
    RustCold,
    RustPreserveNone,
    RustTail,
    Other(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AbiRegisterKind {
    Integer = 1,
    Float = 2,
    Vector = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbiRegisterPlan {
    pub kind: AbiRegisterKind,
    pub bits: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbiUniformPlan {
    pub unit: AbiRegisterPlan,
    pub total_bytes: u64,
    pub consecutive: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbiPassModePlan {
    Ignore,
    Direct,
    Pair,
    Cast {
        pad_i32: bool,
        prefix: Vec<AbiRegisterPlan>,
        rest_offset: Option<u64>,
        rest: AbiUniformPlan,
    },
    Indirect {
        has_metadata: bool,
        on_stack: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbiValuePlan {
    pub size: u64,
    pub align: u64,
    pub mode: AbiPassModePlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FnAbiPlan {
    pub convention: CallingConventionPlan,
    pub variadic: bool,
    pub fixed_count: u32,
    pub can_unwind: bool,
    pub arguments: Vec<AbiValuePlan>,
    pub return_value: AbiValuePlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionPlan {
    pub symbol: String,
    pub abi: FnAbiPlan,
    pub argument_locals: Vec<u32>,
    pub return_local: Option<u32>,
    pub locals: Vec<LocalPlan>,
    pub blocks: Vec<BasicBlockPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternFunctionPlan {
    pub symbol: String,
    pub abi: FnAbiPlan,
    pub argument_types: Vec<ScalarType>,
    pub return_type: Option<ScalarType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BasicBlockPlan {
    pub id: u32,
    pub operations: Vec<Operation>,
    pub terminator: TerminatorPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwitchCasePlan {
    pub value: ValueRef,
    pub target: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminatorPlan {
    Return,
    Goto {
        target: u32,
    },
    Branch {
        condition: ValueRef,
        true_target: u32,
        false_target: u32,
    },
    Assert {
        condition: ValueRef,
        expected: bool,
        target: u32,
        panic_code: u32,
    },
    SwitchInt {
        discr: ValueRef,
        cases: Vec<SwitchCasePlan>,
        otherwise: u32,
    },
    Call {
        callee: String,
        args: Vec<ValueRef>,
        destination: Option<u32>,
        target: u32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetPlan {
    pub triple: String,
    pub object_format: String,
    pub data_layout: String,
    pub pointer_width: u8,
    pub endian: Endian,
    pub cpu: String,
    pub features: String,
    pub relocation_model: String,
    pub code_model: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SciModulePlan {
    pub plan_version: u16,
    pub rustc_commit: String,
    pub target: TargetPlan,
    pub cgu_name: String,
    pub extern_functions: Vec<ExternFunctionPlan>,
    pub functions: Vec<FunctionPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileRequest {
    pub request_id: u64,
    pub output_path: String,
    pub emit_sa_path: Option<String>,
    pub module: SciModulePlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileResponse {
    pub request_id: u64,
    pub success: bool,
    pub diagnostic: String,
}

#[derive(Debug)]
pub enum ProtocolError {
    Io(io::Error),
    InvalidMagic,
    UnsupportedRpcVersion(u16),
    UnsupportedPlanVersion(u16),
    InvalidTag(&'static str, u8),
    InvalidUtf8,
    FrameTooLarge(usize),
    TrailingBytes,
    InvalidData(&'static str),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::InvalidMagic => write!(f, "invalid SCI RPC magic"),
            Self::UnsupportedRpcVersion(version) => {
                write!(f, "unsupported SCI RPC version {version}")
            }
            Self::UnsupportedPlanVersion(version) => {
                write!(f, "unsupported SCI plan version {version}")
            }
            Self::InvalidTag(kind, tag) => write!(f, "invalid {kind} tag {tag}"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 in SCI RPC"),
            Self::FrameTooLarge(size) => write!(f, "SCI RPC frame is too large: {size} bytes"),
            Self::TrailingBytes => write!(f, "trailing bytes in SCI RPC payload"),
            Self::InvalidData(message) => write!(f, "invalid SCI RPC data: {message}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<io::Error> for ProtocolError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub trait WireEncode {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError>;
}

pub trait WireDecode: Sized {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError>;
}

#[derive(Default)]
pub struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn string(&mut self, value: &str) -> Result<(), ProtocolError> {
        let len = u32::try_from(value.len())
            .map_err(|_| ProtocolError::InvalidData("string exceeds u32 length"))?;
        self.u32(len);
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(())
    }

    fn vec<T: WireEncode>(&mut self, values: &[T]) -> Result<(), ProtocolError> {
        let len = u32::try_from(values.len())
            .map_err(|_| ProtocolError::InvalidData("vector exceeds u32 length"))?;
        self.u32(len);
        for value in values {
            value.encode(self)?;
        }
        Ok(())
    }
}

pub struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    pub fn finish(self) -> Result<(), ProtocolError> {
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err(ProtocolError::TrailingBytes)
        }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], ProtocolError> {
        let end = self
            .cursor
            .checked_add(len)
            .ok_or(ProtocolError::InvalidData("cursor overflow"))?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or(ProtocolError::InvalidData("truncated payload"))?;
        self.cursor = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, ProtocolError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("fixed-size slice"),
        ))
    }

    fn u32(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("fixed-size slice"),
        ))
    }

    fn u64(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_le_bytes(
            self.take(8)?.try_into().expect("fixed-size slice"),
        ))
    }

    fn string(&mut self) -> Result<String, ProtocolError> {
        let len = usize::try_from(self.u32()?)
            .map_err(|_| ProtocolError::InvalidData("string length overflow"))?;
        String::from_utf8(self.take(len)?.to_vec()).map_err(|_| ProtocolError::InvalidUtf8)
    }

    fn vec<T: WireDecode>(&mut self) -> Result<Vec<T>, ProtocolError> {
        let len = usize::try_from(self.u32()?)
            .map_err(|_| ProtocolError::InvalidData("vector length overflow"))?;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(T::decode(self)?);
        }
        Ok(values)
    }
}

impl WireEncode for u32 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.u32(*self);
        Ok(())
    }
}

impl WireDecode for u32 {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        decoder.u32()
    }
}

impl WireEncode for u64 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.u64(*self);
        Ok(())
    }
}

impl WireDecode for u64 {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        decoder.u64()
    }
}

fn decode_bool(decoder: &mut Decoder<'_>, field: &'static str) -> Result<bool, ProtocolError> {
    match decoder.u8()? {
        0 => Ok(false),
        1 => Ok(true),
        tag => Err(ProtocolError::InvalidTag(field, tag)),
    }
}

impl<T: WireEncode> WireEncode for Option<T> {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        match self {
            Some(value) => {
                encoder.u8(1);
                value.encode(encoder)?;
            }
            None => encoder.u8(0),
        }
        Ok(())
    }
}

impl<T: WireDecode> WireDecode for Option<T> {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        match decoder.u8()? {
            0 => Ok(None),
            1 => Ok(Some(T::decode(decoder)?)),
            tag => Err(ProtocolError::InvalidTag("option", tag)),
        }
    }
}

impl WireEncode for ScalarType {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.u8(*self as u8);
        Ok(())
    }
}

impl WireDecode for ScalarType {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let tag = decoder.u8()?;
        match tag {
            0 => Ok(Self::I1),
            1 => Ok(Self::I8),
            2 => Ok(Self::I16),
            3 => Ok(Self::I32),
            4 => Ok(Self::I64),
            5 => Ok(Self::U8),
            6 => Ok(Self::U16),
            7 => Ok(Self::U32),
            8 => Ok(Self::U64),
            9 => Ok(Self::Ptr),
            _ => Err(ProtocolError::InvalidTag("scalar type", tag)),
        }
    }
}

impl WireEncode for ValueRef {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        match self {
            Self::Local(local) => {
                encoder.u8(1);
                encoder.u32(*local);
            }
            Self::Integer { ty, bits } => {
                encoder.u8(2);
                ty.encode(encoder)?;
                encoder.u64(*bits);
            }
        }
        Ok(())
    }
}

impl WireDecode for ValueRef {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        match decoder.u8()? {
            1 => Ok(Self::Local(decoder.u32()?)),
            2 => Ok(Self::Integer {
                ty: ScalarType::decode(decoder)?,
                bits: decoder.u64()?,
            }),
            tag => Err(ProtocolError::InvalidTag("value", tag)),
        }
    }
}

impl WireEncode for Operation {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        match self {
            Self::Copy { dst, src } => {
                encoder.u8(1);
                encoder.u32(*dst);
                src.encode(encoder)?;
            }
            Self::Binary { dst, op, lhs, rhs } => {
                encoder.u8(2);
                encoder.u32(*dst);
                encoder.u8(*op as u8);
                lhs.encode(encoder)?;
                rhs.encode(encoder)?;
            }
            Self::Compare { dst, op, lhs, rhs } => {
                encoder.u8(3);
                encoder.u32(*dst);
                encoder.u8(*op as u8);
                lhs.encode(encoder)?;
                rhs.encode(encoder)?;
            }
            Self::Cast { dst, op, src, ty } => {
                encoder.u8(4);
                encoder.u32(*dst);
                encoder.u8(*op as u8);
                src.encode(encoder)?;
                ty.encode(encoder)?;
            }
        }
        Ok(())
    }
}

impl WireDecode for Operation {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        match decoder.u8()? {
            1 => Ok(Self::Copy {
                dst: decoder.u32()?,
                src: ValueRef::decode(decoder)?,
            }),
            2 => {
                let dst = decoder.u32()?;
                let op = match decoder.u8()? {
                    1 => BinaryOp::Add,
                    2 => BinaryOp::Sub,
                    3 => BinaryOp::Mul,
                    4 => BinaryOp::BitXor,
                    5 => BinaryOp::BitAnd,
                    6 => BinaryOp::BitOr,
                    7 => BinaryOp::SDiv,
                    8 => BinaryOp::UDiv,
                    9 => BinaryOp::SRem,
                    10 => BinaryOp::URem,
                    11 => BinaryOp::Shl,
                    12 => BinaryOp::LShr,
                    13 => BinaryOp::AShr,
                    tag => return Err(ProtocolError::InvalidTag("binary operation", tag)),
                };
                Ok(Self::Binary {
                    dst,
                    op,
                    lhs: ValueRef::decode(decoder)?,
                    rhs: ValueRef::decode(decoder)?,
                })
            }
            3 => {
                let dst = decoder.u32()?;
                let op = match decoder.u8()? {
                    1 => CompareOp::Eq,
                    2 => CompareOp::Ne,
                    3 => CompareOp::Slt,
                    4 => CompareOp::Sle,
                    5 => CompareOp::Sgt,
                    6 => CompareOp::Sge,
                    7 => CompareOp::Ult,
                    8 => CompareOp::Ule,
                    9 => CompareOp::Ugt,
                    10 => CompareOp::Uge,
                    tag => return Err(ProtocolError::InvalidTag("compare operation", tag)),
                };
                Ok(Self::Compare {
                    dst,
                    op,
                    lhs: ValueRef::decode(decoder)?,
                    rhs: ValueRef::decode(decoder)?,
                })
            }
            4 => {
                let dst = decoder.u32()?;
                let op = match decoder.u8()? {
                    1 => CastOp::Trunc,
                    2 => CastOp::Zext,
                    3 => CastOp::Sext,
                    4 => CastOp::Bitcast,
                    tag => return Err(ProtocolError::InvalidTag("cast operation", tag)),
                };
                Ok(Self::Cast {
                    dst,
                    op,
                    src: ValueRef::decode(decoder)?,
                    ty: ScalarType::decode(decoder)?,
                })
            }
            tag => Err(ProtocolError::InvalidTag("operation", tag)),
        }
    }
}

impl WireEncode for LocalPlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.u32(self.id);
        self.ty.encode(encoder)
    }
}

impl WireDecode for LocalPlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        Ok(Self {
            id: decoder.u32()?,
            ty: ScalarType::decode(decoder)?,
        })
    }
}

impl WireEncode for CallingConventionPlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        match self {
            Self::C => encoder.u8(1),
            Self::Rust => encoder.u8(2),
            Self::RustCold => encoder.u8(3),
            Self::RustPreserveNone => encoder.u8(4),
            Self::RustTail => encoder.u8(5),
            Self::Other(name) => {
                encoder.u8(6);
                encoder.string(name)?;
            }
        }
        Ok(())
    }
}

impl WireDecode for CallingConventionPlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        match decoder.u8()? {
            1 => Ok(Self::C),
            2 => Ok(Self::Rust),
            3 => Ok(Self::RustCold),
            4 => Ok(Self::RustPreserveNone),
            5 => Ok(Self::RustTail),
            6 => Ok(Self::Other(decoder.string()?)),
            tag => Err(ProtocolError::InvalidTag("calling convention", tag)),
        }
    }
}

impl WireEncode for AbiRegisterPlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.u8(self.kind as u8);
        encoder.u64(self.bits);
        Ok(())
    }
}

impl WireDecode for AbiRegisterPlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let kind = match decoder.u8()? {
            1 => AbiRegisterKind::Integer,
            2 => AbiRegisterKind::Float,
            3 => AbiRegisterKind::Vector,
            tag => return Err(ProtocolError::InvalidTag("ABI register kind", tag)),
        };
        Ok(Self {
            kind,
            bits: decoder.u64()?,
        })
    }
}

impl WireEncode for AbiUniformPlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        self.unit.encode(encoder)?;
        encoder.u64(self.total_bytes);
        encoder.u8(u8::from(self.consecutive));
        Ok(())
    }
}

impl WireDecode for AbiUniformPlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        Ok(Self {
            unit: AbiRegisterPlan::decode(decoder)?,
            total_bytes: decoder.u64()?,
            consecutive: decode_bool(decoder, "ABI uniform consecutive")?,
        })
    }
}

impl WireEncode for AbiPassModePlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        match self {
            Self::Ignore => encoder.u8(1),
            Self::Direct => encoder.u8(2),
            Self::Pair => encoder.u8(3),
            Self::Cast {
                pad_i32,
                prefix,
                rest_offset,
                rest,
            } => {
                encoder.u8(4);
                encoder.u8(u8::from(*pad_i32));
                encoder.vec(prefix)?;
                rest_offset.encode(encoder)?;
                rest.encode(encoder)?;
            }
            Self::Indirect {
                has_metadata,
                on_stack,
            } => {
                encoder.u8(5);
                encoder.u8(u8::from(*has_metadata));
                encoder.u8(u8::from(*on_stack));
            }
        }
        Ok(())
    }
}

impl WireDecode for AbiPassModePlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        match decoder.u8()? {
            1 => Ok(Self::Ignore),
            2 => Ok(Self::Direct),
            3 => Ok(Self::Pair),
            4 => Ok(Self::Cast {
                pad_i32: decode_bool(decoder, "ABI cast padding")?,
                prefix: decoder.vec()?,
                rest_offset: Option::<u64>::decode(decoder)?,
                rest: AbiUniformPlan::decode(decoder)?,
            }),
            5 => Ok(Self::Indirect {
                has_metadata: decode_bool(decoder, "ABI indirect metadata")?,
                on_stack: decode_bool(decoder, "ABI indirect on-stack")?,
            }),
            tag => Err(ProtocolError::InvalidTag("ABI pass mode", tag)),
        }
    }
}

impl WireEncode for AbiValuePlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.u64(self.size);
        encoder.u64(self.align);
        self.mode.encode(encoder)
    }
}

impl WireDecode for AbiValuePlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        Ok(Self {
            size: decoder.u64()?,
            align: decoder.u64()?,
            mode: AbiPassModePlan::decode(decoder)?,
        })
    }
}

impl WireEncode for FnAbiPlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        self.convention.encode(encoder)?;
        encoder.u8(u8::from(self.variadic));
        encoder.u32(self.fixed_count);
        encoder.u8(u8::from(self.can_unwind));
        encoder.vec(&self.arguments)?;
        self.return_value.encode(encoder)
    }
}

impl WireDecode for FnAbiPlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        Ok(Self {
            convention: CallingConventionPlan::decode(decoder)?,
            variadic: decode_bool(decoder, "ABI variadic")?,
            fixed_count: decoder.u32()?,
            can_unwind: decode_bool(decoder, "ABI can-unwind")?,
            arguments: decoder.vec()?,
            return_value: AbiValuePlan::decode(decoder)?,
        })
    }
}

impl WireEncode for FunctionPlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.string(&self.symbol)?;
        self.abi.encode(encoder)?;
        encoder.vec(&self.argument_locals)?;
        self.return_local.encode(encoder)?;
        encoder.vec(&self.locals)?;
        encoder.vec(&self.blocks)
    }
}

impl WireDecode for FunctionPlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        Ok(Self {
            symbol: decoder.string()?,
            abi: FnAbiPlan::decode(decoder)?,
            argument_locals: decoder.vec()?,
            return_local: Option::<u32>::decode(decoder)?,
            locals: decoder.vec()?,
            blocks: decoder.vec()?,
        })
    }
}

impl WireEncode for ExternFunctionPlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.string(&self.symbol)?;
        self.abi.encode(encoder)?;
        encoder.vec(&self.argument_types)?;
        self.return_type.encode(encoder)
    }
}

impl WireDecode for ExternFunctionPlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        Ok(Self {
            symbol: decoder.string()?,
            abi: FnAbiPlan::decode(decoder)?,
            argument_types: decoder.vec()?,
            return_type: Option::<ScalarType>::decode(decoder)?,
        })
    }
}

impl WireEncode for BasicBlockPlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.u32(self.id);
        encoder.vec(&self.operations)?;
        self.terminator.encode(encoder)
    }
}

impl WireDecode for BasicBlockPlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        Ok(Self {
            id: decoder.u32()?,
            operations: decoder.vec()?,
            terminator: TerminatorPlan::decode(decoder)?,
        })
    }
}

impl WireEncode for SwitchCasePlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        self.value.encode(encoder)?;
        encoder.u32(self.target);
        Ok(())
    }
}

impl WireDecode for SwitchCasePlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        Ok(Self {
            value: ValueRef::decode(decoder)?,
            target: decoder.u32()?,
        })
    }
}

impl WireEncode for TerminatorPlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        match self {
            Self::Return => encoder.u8(1),
            Self::Goto { target } => {
                encoder.u8(2);
                encoder.u32(*target);
            }
            Self::Branch {
                condition,
                true_target,
                false_target,
            } => {
                encoder.u8(3);
                condition.encode(encoder)?;
                encoder.u32(*true_target);
                encoder.u32(*false_target);
            }
            Self::Assert {
                condition,
                expected,
                target,
                panic_code,
            } => {
                encoder.u8(5);
                condition.encode(encoder)?;
                encoder.u8(u8::from(*expected));
                encoder.u32(*target);
                encoder.u32(*panic_code);
            }
            Self::SwitchInt {
                discr,
                cases,
                otherwise,
            } => {
                encoder.u8(6);
                discr.encode(encoder)?;
                encoder.vec(cases)?;
                encoder.u32(*otherwise);
            }
            Self::Call {
                callee,
                args,
                destination,
                target,
            } => {
                encoder.u8(4);
                encoder.string(callee)?;
                encoder.vec(args)?;
                destination.encode(encoder)?;
                encoder.u32(*target);
            }
        }
        Ok(())
    }
}

impl WireDecode for TerminatorPlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        match decoder.u8()? {
            1 => Ok(Self::Return),
            2 => Ok(Self::Goto {
                target: decoder.u32()?,
            }),
            3 => Ok(Self::Branch {
                condition: ValueRef::decode(decoder)?,
                true_target: decoder.u32()?,
                false_target: decoder.u32()?,
            }),
            4 => Ok(Self::Call {
                callee: decoder.string()?,
                args: decoder.vec()?,
                destination: Option::<u32>::decode(decoder)?,
                target: decoder.u32()?,
            }),
            5 => Ok(Self::Assert {
                condition: ValueRef::decode(decoder)?,
                expected: match decoder.u8()? {
                    0 => false,
                    1 => true,
                    tag => return Err(ProtocolError::InvalidTag("boolean", tag)),
                },
                target: decoder.u32()?,
                panic_code: decoder.u32()?,
            }),
            6 => Ok(Self::SwitchInt {
                discr: ValueRef::decode(decoder)?,
                cases: decoder.vec()?,
                otherwise: decoder.u32()?,
            }),
            tag => Err(ProtocolError::InvalidTag("terminator", tag)),
        }
    }
}

impl WireEncode for TargetPlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.string(&self.triple)?;
        encoder.string(&self.object_format)?;
        encoder.string(&self.data_layout)?;
        encoder.u8(self.pointer_width);
        encoder.u8(self.endian as u8);
        encoder.string(&self.cpu)?;
        encoder.string(&self.features)?;
        encoder.string(&self.relocation_model)?;
        match &self.code_model {
            Some(code_model) => {
                encoder.u8(1);
                encoder.string(code_model)?;
            }
            None => encoder.u8(0),
        }
        Ok(())
    }
}

impl WireDecode for TargetPlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let triple = decoder.string()?;
        let object_format = decoder.string()?;
        let data_layout = decoder.string()?;
        let pointer_width = decoder.u8()?;
        let endian = match decoder.u8()? {
            1 => Endian::Little,
            2 => Endian::Big,
            tag => return Err(ProtocolError::InvalidTag("endianness", tag)),
        };
        let cpu = decoder.string()?;
        let features = decoder.string()?;
        let relocation_model = decoder.string()?;
        let code_model = match decoder.u8()? {
            0 => None,
            1 => Some(decoder.string()?),
            tag => return Err(ProtocolError::InvalidTag("optional target code model", tag)),
        };
        Ok(Self {
            triple,
            object_format,
            data_layout,
            pointer_width,
            endian,
            cpu,
            features,
            relocation_model,
            code_model,
        })
    }
}

impl WireEncode for SciModulePlan {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.u16(self.plan_version);
        encoder.string(&self.rustc_commit)?;
        self.target.encode(encoder)?;
        encoder.string(&self.cgu_name)?;
        encoder.vec(&self.extern_functions)?;
        encoder.vec(&self.functions)
    }
}

impl WireDecode for SciModulePlan {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let plan_version = decoder.u16()?;
        if plan_version != PLAN_VERSION {
            return Err(ProtocolError::UnsupportedPlanVersion(plan_version));
        }
        Ok(Self {
            plan_version,
            rustc_commit: decoder.string()?,
            target: TargetPlan::decode(decoder)?,
            cgu_name: decoder.string()?,
            extern_functions: decoder.vec()?,
            functions: decoder.vec()?,
        })
    }
}

impl WireEncode for CompileRequest {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.u64(self.request_id);
        encoder.string(&self.output_path)?;
        match &self.emit_sa_path {
            Some(path) => {
                encoder.u8(1);
                encoder.string(path)?;
            }
            None => encoder.u8(0),
        }
        self.module.encode(encoder)
    }
}

impl WireDecode for CompileRequest {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        let request_id = decoder.u64()?;
        let output_path = decoder.string()?;
        let emit_sa_path = match decoder.u8()? {
            0 => None,
            1 => Some(decoder.string()?),
            tag => return Err(ProtocolError::InvalidTag("optional path", tag)),
        };
        Ok(Self {
            request_id,
            output_path,
            emit_sa_path,
            module: SciModulePlan::decode(decoder)?,
        })
    }
}

impl WireEncode for CompileResponse {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), ProtocolError> {
        encoder.u64(self.request_id);
        encoder.u8(u8::from(self.success));
        encoder.string(&self.diagnostic)
    }
}

impl WireDecode for CompileResponse {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ProtocolError> {
        Ok(Self {
            request_id: decoder.u64()?,
            success: match decoder.u8()? {
                0 => false,
                1 => true,
                tag => return Err(ProtocolError::InvalidTag("boolean", tag)),
            },
            diagnostic: decoder.string()?,
        })
    }
}

pub fn encode_payload<T: WireEncode>(value: &T) -> Result<Vec<u8>, ProtocolError> {
    let mut encoder = Encoder::default();
    value.encode(&mut encoder)?;
    Ok(encoder.into_bytes())
}

pub fn decode_payload<T: WireDecode>(bytes: &[u8]) -> Result<T, ProtocolError> {
    let mut decoder = Decoder::new(bytes);
    let value = T::decode(&mut decoder)?;
    decoder.finish()?;
    Ok(value)
}

pub fn write_frame<W: Write, T: WireEncode>(mut writer: W, value: &T) -> Result<(), ProtocolError> {
    let payload = encode_payload(value)?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge(payload.len()));
    }
    writer.write_all(&RPC_MAGIC)?;
    writer.write_all(&RPC_VERSION.to_le_bytes())?;
    writer.write_all(
        &u64::try_from(payload.len())
            .map_err(|_| ProtocolError::FrameTooLarge(payload.len()))?
            .to_le_bytes(),
    )?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

pub fn read_frame<R: Read, T: WireDecode>(mut reader: R) -> Result<T, ProtocolError> {
    let mut magic = [0_u8; RPC_MAGIC.len()];
    reader.read_exact(&mut magic)?;
    if magic != RPC_MAGIC {
        return Err(ProtocolError::InvalidMagic);
    }
    let mut version = [0_u8; 2];
    reader.read_exact(&mut version)?;
    let version = u16::from_le_bytes(version);
    if version != RPC_VERSION {
        return Err(ProtocolError::UnsupportedRpcVersion(version));
    }
    let mut len = [0_u8; 8];
    reader.read_exact(&mut len)?;
    let len = usize::try_from(u64::from_le_bytes(len))
        .map_err(|_| ProtocolError::FrameTooLarge(usize::MAX))?;
    if len > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge(len));
    }
    let mut payload = vec![0; len];
    reader.read_exact(&mut payload)?;
    decode_payload(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn direct_abi(arguments: Vec<(u64, u64)>, return_value: Option<(u64, u64)>) -> FnAbiPlan {
        FnAbiPlan {
            convention: CallingConventionPlan::C,
            variadic: false,
            fixed_count: u32::try_from(arguments.len()).unwrap(),
            can_unwind: false,
            arguments: arguments
                .into_iter()
                .map(|(size, align)| AbiValuePlan {
                    size,
                    align,
                    mode: AbiPassModePlan::Direct,
                })
                .collect(),
            return_value: match return_value {
                Some((size, align)) => AbiValuePlan {
                    size,
                    align,
                    mode: AbiPassModePlan::Direct,
                },
                None => AbiValuePlan {
                    size: 0,
                    align: 1,
                    mode: AbiPassModePlan::Ignore,
                },
            },
        }
    }

    fn sample_request() -> CompileRequest {
        CompileRequest {
            request_id: 42,
            output_path: "/tmp/add.o".into(),
            emit_sa_path: Some("/tmp/add.sa".into()),
            module: SciModulePlan {
                plan_version: PLAN_VERSION,
                rustc_commit: "fcbe7917".into(),
                target: TargetPlan {
                    triple: "x86_64-unknown-linux-gnu".into(),
                    object_format: "elf".into(),
                    data_layout:
                        "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
                            .into(),
                    pointer_width: 64,
                    endian: Endian::Little,
                    cpu: "x86-64".into(),
                    features: String::new(),
                    relocation_model: "pic".into(),
                    code_model: None,
                },
                cgu_name: "add".into(),
                extern_functions: vec![
                    ExternFunctionPlan {
                        symbol: "host_add_i32".into(),
                        abi: direct_abi(vec![(4, 4), (4, 4)], Some((4, 4))),
                        argument_types: vec![ScalarType::I32, ScalarType::I32],
                        return_type: Some(ScalarType::I32),
                    },
                    ExternFunctionPlan {
                        symbol: "host_note_i32".into(),
                        abi: direct_abi(vec![(4, 4)], None),
                        argument_types: vec![ScalarType::I32],
                        return_type: None,
                    },
                    ExternFunctionPlan {
                        symbol: "host_identity_ptr".into(),
                        abi: direct_abi(vec![(8, 8)], Some((8, 8))),
                        argument_types: vec![ScalarType::Ptr],
                        return_type: Some(ScalarType::Ptr),
                    },
                ],
                functions: vec![
                    FunctionPlan {
                        symbol: "add_i32".into(),
                        abi: direct_abi(vec![(4, 4), (4, 4)], Some((4, 4))),
                        argument_locals: vec![1, 2],
                        return_local: Some(0),
                        locals: vec![
                            LocalPlan {
                                id: 0,
                                ty: ScalarType::I32,
                            },
                            LocalPlan {
                                id: 1,
                                ty: ScalarType::I32,
                            },
                            LocalPlan {
                                id: 2,
                                ty: ScalarType::I32,
                            },
                        ],
                        blocks: vec![BasicBlockPlan {
                            id: 0,
                            operations: vec![Operation::Binary {
                                dst: 0,
                                op: BinaryOp::Add,
                                lhs: ValueRef::Local(1),
                                rhs: ValueRef::Local(2),
                            }],
                            terminator: TerminatorPlan::Return,
                        }],
                    },
                    FunctionPlan {
                        symbol: "note_i32".into(),
                        abi: direct_abi(vec![(4, 4)], None),
                        argument_locals: vec![1],
                        return_local: None,
                        locals: vec![LocalPlan {
                            id: 1,
                            ty: ScalarType::I32,
                        }],
                        blocks: vec![
                            BasicBlockPlan {
                                id: 0,
                                operations: Vec::new(),
                                terminator: TerminatorPlan::Call {
                                    callee: "host_note_i32".into(),
                                    args: vec![ValueRef::Local(1)],
                                    destination: None,
                                    target: 1,
                                },
                            },
                            BasicBlockPlan {
                                id: 1,
                                operations: Vec::new(),
                                terminator: TerminatorPlan::Return,
                            },
                        ],
                    },
                ],
            },
        }
    }

    #[test]
    fn payload_round_trip_is_lossless() {
        let request = sample_request();
        let bytes = encode_payload(&request).unwrap();
        assert_eq!(decode_payload::<CompileRequest>(&bytes).unwrap(), request);
    }

    #[test]
    fn framed_round_trip_is_lossless() {
        let request = sample_request();
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &request).unwrap();
        assert_eq!(
            read_frame::<_, CompileRequest>(&bytes[..]).unwrap(),
            request
        );
    }

    #[test]
    fn optional_target_code_model_round_trip_is_lossless() {
        let mut request = sample_request();
        request.module.target.code_model = Some("small".into());
        let bytes = encode_payload(&request).unwrap();
        assert_eq!(decode_payload::<CompileRequest>(&bytes).unwrap(), request);
    }

    #[test]
    fn trailing_payload_is_rejected() {
        let request = sample_request();
        let mut bytes = encode_payload(&request).unwrap();
        bytes.push(0);
        assert!(matches!(
            decode_payload::<CompileRequest>(&bytes),
            Err(ProtocolError::TrailingBytes)
        ));
    }
}
