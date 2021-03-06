//! Generated by `codegen/create_gen_bif.py`
//! Creates a lookup table of BIF functions
//! Config used: OTP20 
#![allow(dead_code)]

use crate::defs::Arity;
use crate::emulator::gen_atoms;
use crate::term::lterm::*;
use crate::bif;


pub struct BifTabItem {
    pub m: LTerm, 
    pub f: LTerm, 
    pub arity: Arity, 
    pub func: bif::BifFn
}


pub static BIF_TABLE: &'static [BifTabItem] = &[

    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::MULTIPLY, arity: 2,
        func: bif::ubif_erlang_multiply_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::SYM_PLUS, arity: 2,
        func: bif::ubif_erlang_plus_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::PLUSPLUS, arity: 2,
        func: bif::bif_erlang_plusplus_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::SYM_MINUS, arity: 2,
        func: bif::ubif_erlang_minus_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::NOTEQUAL, arity: 2,
        func: bif::ubif_erlang_notequal_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::LESSTHAN, arity: 2,
        func: bif::ubif_erlang_lessthan_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::NOTEQUAL_EXACT, arity: 2,
        func: bif::ubif_erlang_notequal_exact_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::EQUAL_EXACT, arity: 2,
        func: bif::ubif_erlang_equal_exact_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::LESSEQUAL, arity: 2,
        func: bif::ubif_erlang_lessequal_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::SYM_EQ_EQ, arity: 2,
        func: bif::ubif_erlang_equalequal_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::GREATERTHAN, arity: 2,
        func: bif::ubif_erlang_greaterthan_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::GREATEREQUAL, arity: 2,
        func: bif::ubif_erlang_greaterequal_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::ATOM_TO_LIST, arity: 1,
        func: bif::bif_erlang_atom_to_list_1 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::ERROR, arity: 1,
        func: bif::bif_erlang_error_1 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::ERROR, arity: 2,
        func: bif::bif_erlang_error_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::HD, arity: 1,
        func: bif::ubif_erlang_hd_1 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::INTEGER_TO_LIST, arity: 1,
        func: bif::bif_erlang_integer_to_list_1 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::IS_BOOLEAN, arity: 1,
        func: bif::ubif_erlang_is_boolean_1 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::IS_PROCESS_ALIVE, arity: 1,
        func: bif::bif_erlang_is_process_alive_1 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::LENGTH, arity: 1,
        func: bif::gcbif_erlang_length_1 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::MAKE_FUN, arity: 3,
        func: bif::bif_erlang_make_fun_3 },
    BifTabItem { m: gen_atoms::LISTS, f: gen_atoms::MEMBER, arity: 2,
        func: bif::bif_lists_member_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::NIF_ERROR, arity: 1,
        func: bif::bif_erlang_nif_error_1 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::NIF_ERROR, arity: 2,
        func: bif::bif_erlang_nif_error_2 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::SELF, arity: 0,
        func: bif::ubif_erlang_self_0 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::SPAWN, arity: 3,
        func: bif::bif_erlang_spawn_3 },
    BifTabItem { m: gen_atoms::ERLANG, f: gen_atoms::TL, arity: 1,
        func: bif::ubif_erlang_tl_1 },
];

