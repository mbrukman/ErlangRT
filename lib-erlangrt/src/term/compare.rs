use crate::{
  emulator::atom,
  fail::RtResult,
  term::{classify, lterm::*},
};
use core::cmp::Ordering;

/// When comparing nested terms they might turn out to be equal. `CompareOp`
/// is stored in `stack` in `eq_terms()` function and tells where to resume
/// comparing the previous term.
#[allow(dead_code)]
enum ContinueCompare {
  // This begins the compare while not knowing types for `a` or `b`.
  AnyType { a: LTerm, b: LTerm },
  // Resume comparing Cons cells, we just reenter `eq_terms_cons`.
  Cons { a: LTerm, b: LTerm },
}

#[allow(dead_code)]
enum EqResult {
  /// Equality result is concluded to be the `bool` value.
  Concluded(Ordering),
  /// Equality is not concluded, but comparing these two terms will give the
  /// result (equivalent to `goto tailrecur_ne` in Erlang/OTP). This happens
  /// when a first element of a nested structure compares equal but some
  /// members remain to be checked recursively.
  CompareNested {
    a: LTerm,
    b: LTerm,
    state: ContinueCompare,
  },
}

/// Compare two terms for equality, fail if types are different even if
/// coercion is otherwise possible.
pub fn cmp_terms(a: LTerm, b: LTerm, exact: bool) -> RtResult<Ordering> {
  // Comparison might want to recurse.
  // To avoid stack growth, do a switch here and continue comparing in a loop.
  // We grow `stack` vector instead of a CPU stack.
  const DEFAULT_CAPACITY: usize = 8;
  let mut stack = Vec::<ContinueCompare>::with_capacity(DEFAULT_CAPACITY);
  let mut op = ContinueCompare::AnyType { a, b };

  // The main comparison loop which is able to step deeper into recursive structures
  loop {
    let eq_result = match op {
      ContinueCompare::AnyType { a: a1, b: b1 }
      | ContinueCompare::Cons { a: a1, b: b1 } => cmp_terms_any_type(a1, b1, exact)?,
    };

    match eq_result {
      EqResult::Concluded(result) if result == Ordering::Equal => {
        if stack.is_empty() {
          // println!("comparison {} {} concluded {:?}", a, b, result);
          return Ok(result);
        } else {
          // println!("comparison {} {} got intermediate result {:?}", a, b, result);
          op = stack.pop().unwrap();
          continue;
        } // stack not empty
      }

      EqResult::Concluded(result) => return Ok(result),

      // Nested terms may accidentally compare equal, to be able to return and
      // continue comparing upper level term, we store a `continue_op` on
      // `stack`.
      EqResult::CompareNested {
        a: a3,
        b: b3,
        state: continue_op,
      } => {
        stack.push(continue_op);
        op = ContinueCompare::AnyType { a: a3, b: b3 };
        continue;
      }
    }
  }
}

/// Given a and b, terms, branch on their type and try do draw some conclusions.
fn cmp_terms_any_type(a: LTerm, b: LTerm, exact: bool) -> RtResult<EqResult> {
  debug_assert!(a.is_value(), "compare_any_type, a NON_VALUE and a {}", b);
  debug_assert!(b.is_value(), "compare_any_type, a {} and a NON_VALUE", a);

  // Compare type tags first
  if a.is_atom() && b.is_atom() {
    return Ok(EqResult::Concluded(cmp_atoms(a, b)));
  }

  // Maybe both a and b are small integers
  let a_is_small = a.is_small();
  let b_is_small = b.is_small();
  if a_is_small && b_is_small {
    let a_small = a.get_small_signed();
    let b_small = b.get_small_signed();
    return Ok(EqResult::Concluded(a_small.cmp(&b_small)));
  }

  // Maybe some of a and b are floats
  let a_is_float = a.is_float();
  let b_is_float = b.is_float();

  // If not exact then allow comparing float to int/bigint
  if !exact && (a_is_float || a_is_small) && (b_is_float || b_is_small) {
    return Ok(EqResult::Concluded(cmp_numbers_not_exact(a, b)));
  } else if a_is_float && b_is_float {
    return Ok(EqResult::Concluded(cmp_floats(a, b)));
  }

  // If types don't compare equal, we can stop comparing here?
  // TODO: Except when we compare numbers
  //  let order = cmp_type_order(a, b);
  //  if order != Ordering::Equal {
  //    return EqResult::Concluded(order);
  //  }

  cmp_terms_primary(a, b, exact)
}

#[inline]
fn cmp_floats(a: LTerm, b: LTerm) -> Ordering {
  // Assume we know both values are floats
  unsafe { cmp_f64_naive(a.get_f64_unsafe(), b.get_f64_unsafe()) }
}

/// Naive f64 comparison, which does not work with NaN and Infinities
#[inline]
fn cmp_f64_naive(a: f64, b: f64) -> Ordering {
  debug_assert!(!a.is_nan());
  debug_assert!(!b.is_nan());
  debug_assert!(!a.is_infinite());
  debug_assert!(!b.is_infinite());

  if a < b {
    return Ordering::Less;
  } else if a > b {
    return Ordering::Greater;
  }
  Ordering::Equal
}

fn cmp_numbers_not_exact(_a: LTerm, _b: LTerm) -> Ordering {
  panic!("TODO: eq_numbers_not_exact")
}

/// Compare two atoms for equality. Returns the ordering result.
fn cmp_atoms(a: LTerm, b: LTerm) -> Ordering {
  assert_ne!(a, LTerm::nil());
  let atomp_a = atom::lookup(a);
  debug_assert!(!atomp_a.is_null(), "cmp_atoms: atom lookup {} failed", a);

  assert_ne!(b, LTerm::nil());
  let atomp_b = atom::lookup(b);
  debug_assert!(!atomp_b.is_null(), "cmp_atoms: atom lookup {} failed", b);

  // This should really be safe, as pointers to Atom exist statically forever
  unsafe {
    let a_len = (*atomp_a).len;
    let b_len = (*atomp_b).len;
    if a_len == b_len {
      return (*atomp_a).name.cmp(&(*atomp_b).name);
    }
    a_len.cmp(&b_len)
  }
}

/// Compare order of two types without looking into their value.
fn cmp_type_order(a: LTerm, b: LTerm) -> Ordering {
  let aclass = classify::classify_term(a);
  let bclass = classify::classify_term(b);
  aclass.cmp(&bclass)
}

/// Switch between comparisons for equality by primary tag (immediate or boxes
/// or fail immediately for different primary tags).
fn cmp_terms_primary(a: LTerm, b: LTerm, exact: bool) -> RtResult<EqResult> {
  //  let a_val = a.raw();
  let a_prim_tag = a.get_term_tag();

  //  let b_val = b.raw();
  let b_prim_tag = b.get_term_tag();
  if b_prim_tag != a_prim_tag {
    // different primary types, compare their classes
    // This can be optimized a little but is there any value in optimization?
    return Ok(EqResult::Concluded(cmp_type_order(a, b)));
  }

  match a_prim_tag {
    TERMTAG_BOXED => {
      if a.is_cp() || b.is_cp() {
        panic!("eq_terms for CP is unsupported")
      }
      cmp_terms_box(a, b)
    }

    TERMTAG_CONS => {
      if !b.is_cons() {
        return Ok(EqResult::Concluded(cmp_mixed_types(a, b)?));
      }

      Ok(unsafe { cmp_cons(a, b) })
    }

    _ => {
      // Any non-boxed compare
      Ok(EqResult::Concluded(cmp_terms_immed(a, b, exact)?))
    } //_ => panic!("Primary tag {:?} eq_terms unsupported", a_prim_tag)
  }
}

// TODO: If this function is used a lot, optimize by doing case on tag bits
fn cmp_terms_immed(a: LTerm, b: LTerm, _exact: bool) -> RtResult<Ordering> {
  //  let av = a.raw();
  //  let bv = b.raw();

  if (a == LTerm::nil() || a == LTerm::empty_tuple() || a == LTerm::empty_binary())
    && (a.raw() == b.raw())
  {
    return Ok(Ordering::Equal);
  }

  if a.is_local_port() {
    if b.is_local_port() {
      panic!("TODO: cmp local vs local port")
    } else if b.is_external_port() {
      panic!("TODO: cmp local vs ext port")
    } else {
      return cmp_mixed_types(a, b);
    }
  }

  if a.is_local_pid() {
    if b.is_local_pid() {
      // Concluded by comparing raw values
      return Ok(a.raw().cmp(&b.raw()));
    } else if b.is_external_pid() {
      panic!("TODO: cmp local vs ext pid")
    } else {
      return cmp_mixed_types(a, b);
    }
  }

  if a.is_boxed() {
    return cmp_terms_immed_box(a, b);
  }

  // if both are internal immediates, compare their raw values or their tags
  if a.is_internal_immediate() && b.is_internal_immediate() {
    let a_tag = a.get_term_tag();
    let b_tag = b.get_term_tag();

    // If tags are same, we can conclude with comparing their values
    if a_tag == b_tag {
      let a_val = a.get_term_val_without_tag();
      let b_val = b.get_term_val_without_tag();
      return Ok(a_val.cmp(&b_val));
    }
    return Ok(a_tag.cmp(&b_tag));
  }

  panic!("TODO: eq_terms_immed {} {}", a, b)
}

#[inline]
fn cmp_terms_immed_box(a: LTerm, b: LTerm) -> RtResult<Ordering> {
  if a.is_tuple() {
    if b.is_tuple() {
      panic!("TODO: cmp tuple vs tuple")
    } else {
      return cmp_mixed_types(a, b);
    }
  } else if a.is_map() {
    if a.is_flat_map() {
      if !b.is_flat_map() {
        if b.is_hash_map() {
          let b_size = b.map_size();
          return Ok(a.map_size().cmp(&b_size));
        }
      } else {
        // Compare two flat maps
        panic!("TODO: cmp flatmap vs flatmap (+exact)")
      }
    } else if !b.is_hash_map() {
      if b.is_flat_map() {
        let b_size = b.map_size();
        return Ok(a.map_size().cmp(&b_size));
      }
    } else {
      // Compare two hash maps
      panic!("TODO: cmp flatmap vs flatmap (+exact)")
    }

  // Hashmap compare strategy:
  // Phase 1. While keys are identical
  //    Do synchronous stepping through leafs of both trees in hash
  //    order. Maintain value compare result of minimal key.
  //
  // Phase 2. If key diff was found in phase 1
  //    Ignore values from now on.
  //    Continue iterate trees by always advancing the one
  //    lagging behind hash-wise. Identical keys are skipped.
  //    A minimal key can only be candidate as tie-breaker if we
  //    have passed that hash value in the other tree (which means
  //    the key did not exist in the other tree).
  } else if a.is_float() {
    if !b.is_float() {
      // TODO: If b is integer and we don't do exact comparison?
      return cmp_mixed_types(a, b);
    } else {
      let a_float = a.get_f64()?;
      let b_float = b.get_f64()?;
      return Ok(a_float.partial_cmp(&b_float).unwrap());
    }
  } else if a.is_big_int() {
    if !b.is_big_int() {
      return cmp_mixed_types(a, b);
    }
  } else if a.is_export() {
    if !b.is_export() {
      return cmp_mixed_types(a, b);
    }
    // Compare two exports: from utils.c line ~2918
    // cmp atoms a.module and b.module
    // cmp atoms a.fn and b.fn
    // cmp arity
    panic!("TODO compare 2 exports")
  } else if a.is_boxed() {
    if !a.is_fun() {
      return cmp_mixed_types(a, b);
    }
    // Compare 2 function objects: from utils.c line ~2937
    // compare a.module, b.module
    // compare old_index
    // compare old_uniq
    // compare num_Free
    panic!("TODO compare 2 fun objects")
  } else if a.is_external_pid() {
    if b.is_local_pid() {
      panic!("TODO compare ext vs local pid")
    } else if b.is_external_pid() {
      panic!("TODO compare ext vs ext pid")
    } else {
      return cmp_mixed_types(a, b);
    }
  } else if a.is_external_port() {
    if b.is_local_port() {
      panic!("TODO compare ext vs local port")
    } else if b.is_external_port() {
      panic!("TODO compare ext vs ext port")
    } else {
      return cmp_mixed_types(a, b);
    }
  } else if a.is_local_ref() {
    if b.is_local_ref() {
      panic!("TODO compare local vs local ref")
    } else if b.is_external_ref() {
      panic!("TODO compare local vs ext ref")
    } else {
      return cmp_mixed_types(a, b);
    }
  } else if a.is_external_ref() {
    if b.is_local_ref() {
      panic!("TODO compare ext vs local ref")
    } else if b.is_external_ref() {
      panic!("TODO compare ext vs ext ref")
    } else {
      return cmp_mixed_types(a, b);
    }
  } else {
    // must be a binary
    assert!(a.is_binary());
    if !b.is_binary() {
      return cmp_mixed_types(a, b);
    }
    panic!("TODO cmp binaries")
  }
  panic!("TODO: eq_terms_immed_box {} {}", a, b)
}

/// Deeper comparison of two values with different types
fn cmp_mixed_types(_a: LTerm, _b: LTerm) -> RtResult<Ordering> {
  panic!("TODO: cmp_mixed_types(a, b)")
}

/// Compare two cons (list) cells.
/// In case when first elements are equal and a deeper comparison is required,
/// we will store the position and return `EqResult::CompareNested`.
/// This will be pushed to a helper stack by the caller (`cmp_terms()`).
/// The function cannot fail.
unsafe fn cmp_cons(a: LTerm, b: LTerm) -> EqResult {
  let mut a_ptr = a.get_cons_ptr();
  let mut b_ptr = b.get_cons_ptr();

  loop {
    // Check the heads
    let a_head = (*a_ptr).hd();
    let b_head = (*b_ptr).hd();

    if !LTerm::is_same(a_head, b_head) {
      // Recurse into a.hd and b.hd, but push a.tl and b.tl to continue
      let continue_op = ContinueCompare::Cons {
        a: (*a_ptr).tl(),
        b: (*b_ptr).tl(),
      };
      return EqResult::CompareNested {
        a: a_head,
        b: b_head,
        state: continue_op,
      };
    }

    // See the tails
    let atl = (*a_ptr).tl();
    let btl = (*b_ptr).tl();

    if LTerm::is_same(atl, btl) {
      return EqResult::Concluded(Ordering::Equal);
    }
    if !atl.is_list() || !btl.is_list() {
      // Just do a regular compare of `a.tl` vs `b.tl`
      let continue_op = ContinueCompare::AnyType { a: atl, b: btl };
      return EqResult::CompareNested {
        a: atl,
        b: btl,
        state: continue_op,
      };
    }

    // Take the next linked cons cell and continue comparing
    a_ptr = atl.get_cons_ptr();
    b_ptr = btl.get_cons_ptr();
  }
}

fn cmp_terms_box(_a: LTerm, _b: LTerm) -> RtResult<EqResult> {
  // TODO: see if cmp_terms_immed_box can be useful
  panic!("TODO: eq_terms_box")
}
