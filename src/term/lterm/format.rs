//! Implement format trait (Display) for LTerm
// Printing low_level Terms as "{}"
use crate::emulator::atom;
use crate::rt_defs::Word;
use crate::term::boxed;
use crate::term::lterm::lterm_impl::*;
use core::fmt;
use core::ptr;

impl fmt::Display for LTerm {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.get_term_tag() {
      TERMTAG_BOXED => unsafe {
        if self.is_cp() {
          write!(f, "CP({:p})", self.get_cp_ptr::<Word>())
        } else {
          let p = self.get_box_ptr();
          format_box_contents(*p, p as *const Word, f)
        }
      },

      TERMTAG_CONS => unsafe {
        if self.cons_is_ascii_string() {
          format_cons_ascii(*self, f)
        } else {
          format_cons(*self, f)
        }
      },
      TERMTAG_SMALL => write!(f, "{}", self.get_small_signed()),
      TERMTAG_SPECIAL => format_special(*self, f),
      TERMTAG_LOCALPID => write!(f, "LocalPid({})", self.get_term_val_without_tag()),
      TERMTAG_LOCALPORT => write!(f, "LocalPort({})", self.get_term_val_without_tag()),
      TERMTAG_ATOM => match atom::to_str(*self) {
        Ok(s) => write!(f, "'{}'", s),
        Err(_e) => write!(f, "Atom?"),
      },
      TERMTAG_HEADER => {
        write!(f, "Header(")?;
        format_box_contents(*self, ptr::null(), f)?;
        write!(f, ")")
      }

      _ => panic!("Primary tag {:?} not recognized", self.get_term_tag()),
    }
  }
} // trait Display


/// Attempt to display contents of a tagged header word and the words which
/// follow it. Arg `p` if not null is used to fetch the following memory words
/// and display more detail.
fn format_box_contents(
  value_at_ptr: LTerm,
  val_ptr: *const Word,
  f: &mut fmt::Formatter,
) -> fmt::Result {
  //    let arity = boxed::headerword_to_arity(value_at_ptr.raw());
  let h_tag = boxed::headerword_to_boxtype(value_at_ptr.raw());

  match h_tag {
    boxed::BOXTYPETAG_BINARY => unsafe {
      let binptr = val_ptr as *const boxed::Binary;
      boxed::Binary::format_binary(binptr, f)
    },
    boxed::BOXTYPETAG_TUPLE => unsafe { format_tuple(val_ptr, f) },
    boxed::BOXTYPETAG_CLOSURE => write!(f, "Fun<>"),
    boxed::BOXTYPETAG_FLOAT => unsafe {
      let fptr = val_ptr as *const boxed::Float;
      write!(f, "{}", (*fptr).value)
    },
    boxed::BOXTYPETAG_EXTERNALPID => write!(f, "ExtPid"),
    boxed::BOXTYPETAG_EXTERNALPORT => write!(f, "ExtPort"),
    boxed::BOXTYPETAG_EXTERNALREF => write!(f, "ExtRef"),

    _ => panic!("Unexpected header tag {:?}", h_tag),
  }
}

//
// Formatting helpers
//

fn format_special(term: LTerm, f: &mut fmt::Formatter) -> fmt::Result {
  match term.get_special_tag() {
    SPECIALTAG_CONST => {
      if term == LTerm::nil() {
        return write!(f, "[]");
      } else if term.is_non_value() {
        return write!(f, "NON_VALUE");
      } else if term == LTerm::empty_binary() {
        return write!(f, "<<>>");
      } else if term == LTerm::empty_tuple() {
        return write!(f, "{{}}");
      }
    }
    SPECIALTAG_REGX => return write!(f, "X{}", term.get_special_value()),
    SPECIALTAG_REGY => return write!(f, "Y{}", term.get_special_value()),
    SPECIALTAG_REGFP => return write!(f, "F{}", term.get_special_value()),
    SPECIALTAG_OPCODE => return write!(f, "Opcode({})", term.get_special_value()),
    _ => {}
  }
  write!(
    f,
    "Special(0x{:x}; 0x{:x})",
    term.get_special_tag().0,
    term.get_special_value()
  )
}

/// Given `p`, a pointer to tuple header word, format tuple contents.
unsafe fn format_tuple(p: *const Word, f: &mut fmt::Formatter) -> fmt::Result {
  let tptr = match boxed::Tuple::from_pointer(p) {
    Ok(x) => x,
    Err(e) => return write!(f, "<err formatting tuple: {:?}>", e),
  };

  write!(f, "{{")?;

  let arity = boxed::Tuple::get_arity(tptr);
  for i in 0..arity {
    write!(f, "{}", boxed::Tuple::get_element_base0(tptr, i))?;
    if i < arity - 1 {
      write!(f, ", ")?
    }
  }
  write!(f, "}}")
}


pub unsafe fn format_cons(term: LTerm, f: &mut fmt::Formatter) -> fmt::Result {
  write!(f, "[")?;

  let mut raw_cons = term.get_cons_ptr();
  loop {
    write!(f, "{}", (*raw_cons).hd())?;
    let tl = (*raw_cons).tl();
    if tl == LTerm::nil() {
      // Proper list ends here, do not show the tail
      break;
    } else if tl.is_cons() {
      // List continues, print a comma and follow the tail
      write!(f, ", ")?;
      raw_cons = tl.get_cons_ptr();
    } else {
      // Improper list, show tail
      write!(f, "| {}", tl)?;
      break;
    }
  }
  write!(f, "]")
}


pub unsafe fn format_cons_ascii(term: LTerm, f: &mut fmt::Formatter) -> fmt::Result {
  write!(f, "\"")?;

  let mut raw_cons = term.get_cons_ptr();
  loop {
    write!(f, "{}", (*raw_cons).hd().get_small_unsigned() as u8 as char)?;
    let tl = (*raw_cons).tl();
    if tl == LTerm::nil() {
      // Proper list ends here, do not show the tail
      break;
    } else if tl.is_cons() {
      // List continues, follow the tail
      raw_cons = tl.get_cons_ptr();
    } else {
      // Improper list, must not happen because we checked for proper NIL
      // tail in cons_is_ascii_string. Let's do some panic!
      panic!("Printing an improper list as ASCII string")
    }
  }
  write!(f, "\"")
}
