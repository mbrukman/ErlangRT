use super::Context;
use crate::{
  beam::disp_result::DispatchResult,
  bif::{self, BifFn},
  emulator::{mfa::MFArity, process::Process, vm::VM},
  fail::{self, Error, RtResult},
  term::{boxed::import, lterm::*},
};
use core::slice;

// fn module() -> &'static str { "runtime_ctx.call_bif: " }

// Call Bif generic facilities
//

/// A Bif can be referenced by an import `LTerm`, an `MFArity`...
#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum CallBifTarget {
  /// A term containing pointer to a `boxed::Import`.
  ImportTerm(LTerm),
  /// A const pointer to an `import::Import`.
  ImportPointer(*const import::Import),
  /// An MFA reference which needs to be resolved.
  MFArity(MFArity),
  /// A resolved pointer to a `BifFn`.
  BifFnPointer(BifFn),
}

/// Generic bif0,1,2 application. Bif0 cannot have a fail label but bif1 and
/// bif2 can, so on exception a jump will be performed.
///
/// Args:
///   `curr_p` - the process which is running;
///   `target` - the term pointing to a callable;
///   `fail_label` - if not NIL, we suppress a possible exception and jump there;
///   `args` - the arguments;
///   `dst` - register where the result will go;
///   `gc` if gc is allowed then `ctx.live` will be used as live.
#[inline]
pub fn find_and_call_bif(
  vm: &mut VM,
  ctx: &mut Context,
  curr_p: &mut Process,
  fail_label: LTerm,
  target: CallBifTarget,
  args: &[LTerm],
  dst: LTerm,
  gc: bool,
) -> RtResult<DispatchResult> {
  // Try resolve BIF destination, which can be defined by an import, mfarity
  // a pointer to import, or a pointer to bif function.
  // TODO: Maybe make this use codeserver generic lookup_mfa or extend it to support this
  let maybe_bif_fn = match target {
    CallBifTarget::ImportTerm(ho_imp) => callbif_resolve_import(ho_imp, args.len())?,

    CallBifTarget::MFArity(mfa) => callbif_resolve_mfa(&mfa)?,

    CallBifTarget::ImportPointer(ho_imp_ptr) => {
      let fn_ptr = unsafe { (*ho_imp_ptr).resolve_bif()? };
      BifResolutionResult::FnPointer(fn_ptr)
    }

    CallBifTarget::BifFnPointer(fn_ptr) => BifResolutionResult::FnPointer(fn_ptr),
  };

  // Now having resolved the bif function, let's call it
  let bif_result = match maybe_bif_fn {
    BifResolutionResult::FnPointer(fn_ptr) => call_bif_fn(vm, ctx, curr_p, fn_ptr, args),

    BifResolutionResult::BadfunError(badfun_val) => {
      return fail::create::badfun_val(badfun_val, &mut curr_p.heap);
    }
  };

  // Now having called the function let's see if there was some good result or
  // an error occured

  // On error and if fail label is a CP, perform a goto
  // Assume that error is already written to `reason` in process
  match bif_result {
    Err(Error::Exception(_, _)) => {
      if fail_label.is_cp() {
        ctx.jump(fail_label)
      }
      // Set exception via dispatchresult; pass through the error
      Err(bif_result.unwrap_err())
    }
    Err(_) => {
      // pass through the error
      Err(bif_result.unwrap_err())
    }
    Ok(val) => {
      println!("call_bif a={} gc={} call result {}", args.len(), gc, val);
      // if dst is not NIL, store the result in it
      if dst != LTerm::nil() {
        ctx.store_value(val, dst, &mut curr_p.heap)?;
      }
      Ok(DispatchResult::Normal)
    }
  }
}

//#[inline]
// fn callbif_handle_fail(e: &fail::Error) -> Hopefully<DispatchResult> {
//  panic!("{}bif call failed with {:?}", module(), e)
//}

#[allow(dead_code)]
enum BifResolutionResult {
  FnPointer(BifFn),
  BadfunError(LTerm),
}

/// Given a term with import, resolve it to a bif function pointer or fail.
/// Arg: check_arity - performs check of args count vs function arity
/// Return: A bif function or an error
fn callbif_resolve_import(
  imp: LTerm,
  check_arity: usize,
) -> RtResult<BifResolutionResult> {
  // Possibly a boxed::Import object on heap which contains m:f/arity
  let imp_p = imp.get_box_ptr_safe::<import::Import>()?;
  assert_eq!(unsafe { (*imp_p).mfarity.arity }, check_arity);

  // Here HOImport pointer is found, try and resolve it to a Rust function ptr
  let fn_ptr = unsafe { (*imp_p).resolve_bif()? };
  Ok(BifResolutionResult::FnPointer(fn_ptr))
}

/// Simply maps Ok/Err from `find_bif` to `BifResolutionResult`.
// TODO: Remove this and call find_bif directly
#[inline]
fn callbif_resolve_mfa(mfa: &MFArity) -> RtResult<BifResolutionResult> {
  Ok(BifResolutionResult::FnPointer(bif::find_bif(&mfa)?))
}

/// Given a bif function pointer and args with possibly register/slot values
/// in them, first resolve these args to values, and then call the function
// #[inline]
pub fn call_bif_fn(
  vm: &mut VM,
  ctx: &mut Context,
  curr_p: &mut Process,
  func_pointer: BifFn,
  args: &[LTerm],
) -> RtResult<LTerm> {
  let n_args = args.len();

  // Make a slice from the args. Bif arg count can go up to 3
  assert!(args.len() < 4);
  let mut loaded_args = [LTerm::nil(); 4];

  {
    let heap = &curr_p.heap;
    for i in 0..n_args {
      loaded_args[i] = ctx.load(args[i], heap);
    }
  }

  // Take n_args elements from args
  let loaded_args1 = unsafe { slice::from_raw_parts(&loaded_args[0], n_args) };

  // Apply the BIF call and return BifResult
  (func_pointer)(vm, curr_p, loaded_args1)
}
