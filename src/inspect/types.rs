//! Data types used to describe runtime Python types.

use std::array;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};

/// Designation of a Python type.
///
/// This enum is used to handle advanced types, such as types with generics.
/// Its [`Display`] implementation can be used to convert to the type hint notation (e.g. `List[int]`).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TypeInfo {
    /// The type `typing.Any`, which represents any possible value (unknown type).
    Any,
    /// The type `typing.None`.
    None,
    /// The type `typing.NoReturn`, which represents functions that never return (they can still panic / throw, similar to `never` in Rust).
    NoReturn,
    /// The type `typing.Callable`.
    ///
    /// The first argument represents the parameters of the callable:
    /// - `Some` of a vector of types to represent the signature,
    /// - `None` if the signature is unknown (allows any number of arguments with type `Any`).
    ///
    /// The second argument represents the return type.
    Callable(Option<Vec<TypeInfo>>, Box<TypeInfo>),
    /// The type `typing.Callable`.
    ///
    /// The first argument represents the parameters of the callable:
    /// - `Some` of a vector of types to represent the signature,
    /// - `None` if the signature is unknown (allows any number of arguments with type `Any`).
    ///
    /// The second argument represents the return type.
    CallableStatic(Option<&'static [TypeInfo]>, &'static TypeInfo),
    /// The type `typing.tuple`.
    ///
    /// The argument represents the contents of the tuple:
    /// - `Some` of a vector of types to represent the accepted types,
    /// - `Some` of an empty vector for the empty tuple,
    /// - `None` if the number and type of accepted values is unknown.
    ///
    /// If the number of accepted values is unknown, but their type is, use [`Self::UnsizedTypedTuple`].
    Tuple(Option<Cow<'static, [TypeInfo]>>),
    /// The type `typing.Tuple`.
    ///
    /// Use this variant to represent a tuple of unknown size but of known types.
    ///
    /// If the type is unknown, or if the number of elements is known, use [`Self::Tuple`].
    UnsizedTypedTuple(Box<TypeInfo>),
    /// A Python class.
    Class {
        /// The module this class comes from.
        module: ModuleName,
        /// The name of this class, as it appears in a type hint.
        name: Cow<'static, str>,
        /// The generics accepted by this class (empty vector if this class is not generic).
        type_vars: Cow<'static, [TypeInfo]>,
    },
}

/// Declares which module a type is a part of.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ModuleName {
    /// The type is built-in: it doesn't need to be imported.
    Builtin,
    /// The type is in the current module: it doesn't need to be imported in this module, but needs to be imported in others.
    CurrentModule,
    /// The type is in the specified module.
    Module(Cow<'static, str>),
}

impl TypeInfo {
    /// Returns the module in which a type is declared.
    ///
    /// Returns `None` if the type is declared in the current module.
    pub fn module_name(&self) -> Option<&str> {
        match self {
            TypeInfo::Any
            | TypeInfo::None
            | TypeInfo::NoReturn
            | TypeInfo::CallableStatic(_, _)
            | TypeInfo::Callable(_, _)
            | TypeInfo::Tuple(_)
            | TypeInfo::UnsizedTypedTuple(_) => Some("typing"),
            TypeInfo::Class { module, .. } => match module {
                ModuleName::Builtin => Some("builtins"),
                ModuleName::CurrentModule => None,
                ModuleName::Module(name) => Some(name),
            },
        }
    }

    /// Returns the name of a type.
    ///
    /// The name of a type is the part of the hint that is not generic (e.g. `List` instead of `List[int]`).
    pub fn name(&self) -> Cow<'_, str> {
        Cow::from(match self {
            TypeInfo::Any => "Any",
            TypeInfo::None => "None",
            TypeInfo::NoReturn => "NoReturn",
            TypeInfo::CallableStatic(_, _) => "Callable",
            TypeInfo::Callable(_, _) => "Callable",
            TypeInfo::Tuple(_) => "Tuple",
            TypeInfo::UnsizedTypedTuple(_) => "Tuple",
            TypeInfo::Class { name, .. } => name,
        })
    }
}

// Utilities for easily instantiating TypeInfo structures for built-in/common types.
impl TypeInfo {
    /// The Python `Optional` type.
    pub fn optional_of(t: TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Optional"),
            type_vars: Cow::Owned(vec![t]),
        }
    }

    /// The Python `Optional` type.
    pub const fn optional_of_const(t: &'static TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Optional"),
            type_vars: Cow::Borrowed(array::from_ref(t)),
        }
    }

    /// The Python `Union` type.
    pub fn union_of(types: &[TypeInfo]) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Union"),
            type_vars: Cow::Owned(types.to_vec()),
        }
    }

    /// The Python `Union` type.
    pub const fn union_of_const(types: &'static [TypeInfo]) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Union"),
            type_vars: Cow::Borrowed(types),
        }
    }

    /// The Python `List` type.
    pub fn list_of(t: TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("List"),
            type_vars: Cow::Owned(vec![t]),
        }
    }

    /// The Python `List` type.
    pub const fn list_of_const(t: &'static TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("List"),
            type_vars: Cow::Borrowed(array::from_ref(t)),
        }
    }

    /// The Python `Sequence` type.
    pub fn sequence_of(t: TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Sequence"),
            type_vars: Cow::Owned(vec![t]),
        }
    }

    /// The Python `Sequence` type.
    pub const fn sequence_of_const(t: &'static TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Sequence"),
            type_vars: Cow::Borrowed(array::from_ref(t)),
        }
    }

    /// The Python `Set` type.
    pub fn set_of(t: TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Set"),
            type_vars: Cow::Owned(vec![t]),
        }
    }

    /// The Python `Set` type.
    pub const fn set_of_const(t: &'static TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Set"),
            type_vars: Cow::Borrowed(array::from_ref(t)),
        }
    }

    /// The Python `FrozenSet` type.
    pub fn frozen_set_of(t: TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("FrozenSet"),
            type_vars: Cow::Owned(vec![t]),
        }
    }

    /// The Python `FrozenSet` type.
    pub const fn frozen_set_of_const(t: &'static TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("FrozenSet"),
            type_vars: Cow::Borrowed(array::from_ref(t)),
        }
    }

    /// The Python `Iterable` type.
    pub fn iterable_of(t: TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Iterable"),
            type_vars: Cow::Owned(vec![t]),
        }
    }

    /// The Python `Iterable` type.
    pub const fn iterable_of_const(t: &'static TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Iterable"),
            type_vars: Cow::Borrowed(array::from_ref(t)),
        }
    }

    /// The Python `Iterator` type.
    pub fn iterator_of(t: TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Iterator"),
            type_vars: Cow::Owned(vec![t]),
        }
    }

    /// The Python `Iterator` type.
    pub const fn iterator_of_const(t: &'static TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Iterator"),
            type_vars: Cow::Borrowed(array::from_ref(t)),
        }
    }

    /// The Python `Dict` type.
    pub fn dict_of(key: TypeInfo, value: TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Dict"),
            type_vars: Cow::Owned(vec![key, value]),
        }
    }

    /// The Python `Dict` type.
    pub const fn dict_of_const(types @ [_key, _value]: &'static [TypeInfo; 2]) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Dict"),
            type_vars: Cow::Borrowed(types),
        }
    }

    /// The Python `Mapping` type.
    pub fn mapping_of(key: TypeInfo, value: TypeInfo) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Mapping"),
            type_vars: Cow::Owned(vec![key, value]),
        }
    }

    /// The Python `Mapping` type.
    pub const fn mapping_of_const(types @ [_key, _value]: &'static [TypeInfo; 2]) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed("typing")),
            name: Cow::Borrowed("Mapping"),
            type_vars: Cow::Borrowed(types),
        }
    }

    /// Convenience factory for non-generic builtins (e.g. `int`).
    pub const fn builtin(name: &'static str) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Builtin,
            name: Cow::Borrowed(name),
            type_vars: Cow::Borrowed(&[]),
        }
    }

    /// Convenience factory for non-generic classes.
    pub const fn class(module: &'static str, name: &'static str) -> TypeInfo {
        TypeInfo::Class {
            module: ModuleName::Module(Cow::Borrowed(module)),
            name: Cow::Borrowed(name),
            type_vars: Cow::Borrowed(&[]),
        }
    }

    /// Convenience factory for callable types.
    pub fn callable(args: Option<Vec<TypeInfo>>, output: TypeInfo) -> TypeInfo {
        TypeInfo::Callable(
            args,
            Box::new(output)
        )
    }

    /// Convenience factory for callable types.
    pub const fn callable_const(args: Option<&'static [TypeInfo]>, output: &'static TypeInfo) -> TypeInfo {
        TypeInfo::CallableStatic(
            args,
            output
        )
    }
}

impl Display for TypeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeInfo::Any | TypeInfo::None | TypeInfo::NoReturn => write!(f, "{}", self.name()),
            TypeInfo::Callable(input, output) => {
                write!(f, "Callable[")?;

                if let Some(input) = input {
                    write!(f, "[")?;
                    let mut comma = false;
                    for arg in input.iter() {
                        if comma {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                        comma = true;
                    }
                    write!(f, "]")?;
                } else {
                    write!(f, "...")?;
                }

                write!(f, ", {}]", output)
            }
            TypeInfo::CallableStatic(other, output) => {todo!()}
            TypeInfo::Tuple(types) => {
                write!(f, "Tuple[")?;

                if let Some(types) = types {
                    if types.is_empty() {
                        write!(f, "()")?;
                    } else {
                        let mut comma = false;
                        for t in types.iter() {
                            if comma {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}", t)?;
                            comma = true;
                        }
                    }
                } else {
                    write!(f, "...")?;
                }

                write!(f, "]")
            }
            TypeInfo::UnsizedTypedTuple(t) => write!(f, "Tuple[{}, ...]", t),
            TypeInfo::Class {
                name, type_vars, ..
            } => {
                write!(f, "{}", name)?;

                if !type_vars.is_empty() {
                    write!(f, "[")?;

                    let mut comma = false;
                    for var in type_vars.iter() {
                        if comma {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", var)?;
                        comma = true;
                    }

                    write!(f, "]")
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;

    use crate::inspect::types::{ModuleName, TypeInfo};

    #[track_caller]
    pub fn assert_display(t: &TypeInfo, expected: &str) {
        assert_eq!(format!("{}", t), expected)
    }

    #[test]
    fn basic() {
        assert_display(&TypeInfo::Any, "Any");
        assert_display(&TypeInfo::None, "None");
        assert_display(&TypeInfo::NoReturn, "NoReturn");

        assert_display(&TypeInfo::builtin("int"), "int");
    }

    #[test]
    fn callable() {
        let any_to_int = TypeInfo::Callable(None, Box::new(TypeInfo::builtin("int")));
        assert_display(&any_to_int, "Callable[..., int]");

        let sum = TypeInfo::Callable(
            Some(vec![TypeInfo::builtin("int"), TypeInfo::builtin("int")]),
            Box::new(TypeInfo::builtin("int")),
        );
        assert_display(&sum, "Callable[[int, int], int]");
    }

    #[test]
    fn tuple() {
        let any = TypeInfo::Tuple(None);
        assert_display(&any, "Tuple[...]");

        let triple = TypeInfo::Tuple(Some(vec![
            TypeInfo::builtin("int"),
            TypeInfo::builtin("str"),
            TypeInfo::builtin("bool"),
        ].into()));
        assert_display(&triple, "Tuple[int, str, bool]");

        let empty = TypeInfo::Tuple(Some(vec![].into()));
        assert_display(&empty, "Tuple[()]");

        let typed = TypeInfo::UnsizedTypedTuple(Box::new(TypeInfo::builtin("bool")));
        assert_display(&typed, "Tuple[bool, ...]");
    }

    #[test]
    fn class() {
        let class1 = TypeInfo::Class {
            module: ModuleName::CurrentModule,
            name: Cow::from("MyClass"),
            type_vars: Cow::Borrowed(&[]),
        };
        assert_display(&class1, "MyClass");

        let class2 = TypeInfo::Class {
            module: ModuleName::CurrentModule,
            name: Cow::from("MyClass"),
            type_vars: vec![TypeInfo::builtin("int"), TypeInfo::builtin("bool")].into(),
        };
        assert_display(&class2, "MyClass[int, bool]");
    }

    #[test]
    fn collections() {
        let int = TypeInfo::builtin("int");
        let bool = TypeInfo::builtin("bool");
        let str = TypeInfo::builtin("str");

        let list = TypeInfo::list_of(int.clone());
        assert_display(&list, "List[int]");

        let sequence = TypeInfo::sequence_of(bool.clone());
        assert_display(&sequence, "Sequence[bool]");

        let optional = TypeInfo::optional_of(str.clone());
        assert_display(&optional, "Optional[str]");

        let iterable = TypeInfo::iterable_of(int.clone());
        assert_display(&iterable, "Iterable[int]");

        let iterator = TypeInfo::iterator_of(bool);
        assert_display(&iterator, "Iterator[bool]");

        let dict = TypeInfo::dict_of(int.clone(), str.clone());
        assert_display(&dict, "Dict[int, str]");

        let mapping = TypeInfo::mapping_of(int, str.clone());
        assert_display(&mapping, "Mapping[int, str]");

        let set = TypeInfo::set_of(str.clone());
        assert_display(&set, "Set[str]");

        let frozen_set = TypeInfo::frozen_set_of(str);
        assert_display(&frozen_set, "FrozenSet[str]");
    }

    #[test]
    fn complicated() {
        let int = TypeInfo::builtin("int");
        assert_display(&int, "int");

        let bool = TypeInfo::builtin("bool");
        assert_display(&bool, "bool");

        let str = TypeInfo::builtin("str");
        assert_display(&str, "str");

        let any = TypeInfo::Any;
        assert_display(&any, "Any");

        let params = TypeInfo::union_of(&[int.clone(), str]);
        assert_display(&params, "Union[int, str]");

        let func = TypeInfo::Callable(Some(vec![params, any]), Box::new(bool));
        assert_display(&func, "Callable[[Union[int, str], Any], bool]");

        let dict = TypeInfo::mapping_of(int, func);
        assert_display(
            &dict,
            "Mapping[int, Callable[[Union[int, str], Any], bool]]",
        );
    }
}

#[cfg(test)]
mod conversion {
    use std::collections::{HashMap, HashSet};

    use crate::inspect::types::test::assert_display;
    use crate::{FromPyObject, IntoPyObject};

    #[test]
    fn unsigned_int() {
        assert_display(&usize::TYPE_OUTPUT, "int");
        assert_display(&usize::TYPE_INPUT, "int");

        assert_display(&u8::TYPE_OUTPUT, "int");
        assert_display(&u8::TYPE_INPUT, "int");

        assert_display(&u16::TYPE_OUTPUT, "int");
        assert_display(&u16::TYPE_INPUT, "int");

        assert_display(&u32::TYPE_OUTPUT, "int");
        assert_display(&u32::TYPE_INPUT, "int");

        assert_display(&u64::TYPE_OUTPUT, "int");
        assert_display(&u64::TYPE_INPUT, "int");
    }

    #[test]
    fn signed_int() {
        assert_display(&isize::TYPE_OUTPUT, "int");
        assert_display(&isize::TYPE_INPUT, "int");

        assert_display(&i8::TYPE_OUTPUT, "int");
        assert_display(&i8::TYPE_INPUT, "int");

        assert_display(&i16::TYPE_OUTPUT, "int");
        assert_display(&i16::TYPE_INPUT, "int");

        assert_display(&i32::TYPE_OUTPUT, "int");
        assert_display(&i32::TYPE_INPUT, "int");

        assert_display(&i64::TYPE_OUTPUT, "int");
        assert_display(&i64::TYPE_INPUT, "int");
    }

    #[test]
    fn float() {
        assert_display(&f32::TYPE_OUTPUT, "float");
        assert_display(&f32::TYPE_INPUT, "float");

        assert_display(&f64::TYPE_OUTPUT, "float");
        assert_display(&f64::TYPE_INPUT, "float");
    }

    #[test]
    fn bool() {
        assert_display(&bool::TYPE_OUTPUT, "bool");
        assert_display(&bool::TYPE_INPUT, "bool");
    }

    #[test]
    fn text() {
        assert_display(&String::TYPE_OUTPUT, "str");
        assert_display(&String::TYPE_INPUT, "str");

        assert_display(&<&[u8]>::TYPE_OUTPUT, "Union[bytes, List[int]]");
        assert_display(&<&[String]>::TYPE_OUTPUT, "Union[bytes, List[str]]");
        assert_display(
            &<&[u8] as crate::conversion::FromPyObjectBound>::TYPE_INPUT,
            "bytes",
        );
    }

    #[test]
    fn collections() {
        assert_display(&<Vec<usize>>::TYPE_OUTPUT, "List[int]");
        assert_display(&<Vec<usize>>::TYPE_INPUT, "Sequence[int]");

        assert_display(&<HashSet<usize>>::TYPE_OUTPUT, "Set[int]");
        assert_display(&<HashSet<usize>>::TYPE_INPUT, "Set[int]");

        assert_display(&<HashMap<usize, f32>>::TYPE_OUTPUT, "Dict[int, float]");
        assert_display(&<HashMap<usize, f32>>::TYPE_INPUT, "Mapping[int, float]");

        assert_display(&<(usize, f32)>::TYPE_INPUT, "Tuple[int, float]");
    }
}
