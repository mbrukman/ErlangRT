//! Module defines Runtime Context which represents the low-level VM state of
//! a running process, such as registers, code pointer, etc.
use crate::{
  defs::{Reductions, Word, MAX_FPREGS, MAX_XREGS},
  emulator::{
    code::{opcode, CodePtr},
    code_srv::MFALookupResult,
    heap,
    process::Process,
    vm::VM,
  },
  fail::RtResult,
  term::lterm::{
    LTerm, SpecialTag, SPECIALTAG_REGFP, SPECIALTAG_REGX, SPECIALTAG_REGY,
    TERMTAG_SPECIAL,
  },
};
use colored::Colorize;
use core::{fmt, slice};

pub mod call_bif;
pub mod call_closure;
pub mod call_export;

fn module() -> &'static str {
  "runtime_ctx: "
}

/// Structure represents the runtime state of a VM process. It is "swapped in"
/// when the process is about to run, and "swapped out", when the process is
/// done running its time-slice.
pub struct Context {
  /// Current code location, const ptr (unsafe!).
  pub ip: CodePtr,

  /// Return location, for one return without using the stack.
  pub cp: CodePtr,

  /// A metric of CPU time spent on running the code, roughly equal to 1 function call
  pub reductions: isize,

  /// Current state of X registers.
  regs: [LTerm; MAX_XREGS],

  /// How many X registers are currently used.
  pub live: usize,

  /// Current state of Y registers.
  pub fpregs: [f64; MAX_FPREGS],
}

impl Context {
  pub fn new(ip: CodePtr) -> Context {
    Context {
      cp: CodePtr::null(),
      fpregs: [0.0; MAX_FPREGS],
      ip,
      regs: [LTerm::non_value(); MAX_XREGS],
      live: 0,
      reductions: 0,
    }
  }

  /// Read contents of an X register.
  #[inline]
  pub fn get_x(&self, index: usize) -> LTerm {
    // debug_assert!(index < self.live);
    let result = self.regs[index];
    debug_assert!(result.is_value(), "Should never get a NON_VALUE from x[]");
    result
  }

  #[inline]
  pub fn set_x(&mut self, index: usize, val: LTerm) {
    if cfg!(feature = "trace_register_changes") {
      println!("{}{} = {}", "set x".blue(), index, val);
    }
    // debug_assert!(val.is_value(), "Should never set x[] to a NON_VALUE");
    self.regs[index] = val;
  }

  #[inline]
  pub fn swap_in(&mut self) {
    // This amount is RESET every time process is about to be scheduled in, i.e.
    // there can be no "debt" of reductions, but the idea is nice.
    self.reductions = Reductions::DEFAULT;
  }

  #[inline]
  pub fn fetch_opcode(&mut self) -> opcode::RawOpcode {
    self.reductions -= Reductions::FETCH_OPCODE_COST;
    opcode::from_memory_word(self.fetch())
  }

  /// Read a word from `self.ip` and advance `ip` by 1 word.
  /// NOTE: The compiler seems to be smart enough to optimize multiple fetches
  /// as multiple reads and a single increment.
  #[inline]
  pub fn fetch(&mut self) -> Word {
    unsafe {
      let w = *(self.ip.get_pointer());
      self.ip.offset(1);
      w
    }
  }

  /// Step back one word in the code.
  pub fn unfetch(&mut self) {
    self.ip.offset(-1);
  }

  /// Fetch a word from code, assume it is an `LTerm`. The code position is
  /// advanced by 1.
  #[inline]
  pub fn fetch_term(&mut self) -> LTerm {
    LTerm::from_raw(self.fetch())
  }

  /// Using current position in code as the starting address, create a new
  /// `&[LTerm]` slice of given length and advance the read pointer. This is
  /// used for fetching arrays of args from code without moving them.
  pub fn fetch_slice(&mut self, sz: usize) -> &'static [LTerm] {
    let ip0 = self.ip.get_pointer() as *const LTerm;
    unsafe {
      self.ip.offset(sz as isize);
      slice::from_raw_parts(ip0, sz)
    }
  }

  /// Returns slice of registers `offset` to `sz`, bypassing the borrow checker
  /// It is the caller responsibility to forget the registers slice ASAP.
  pub fn registers_slice(&mut self, offset: usize, sz: usize) -> &'static [LTerm] {
    unsafe { slice::from_raw_parts(self.regs.as_ptr().add(offset), sz) }
  }

  /// Returns mutable slice of registers `offset` to `sz`, bypassing the borrow checker
  /// It is the caller responsibility to forget the registers slice ASAP.
  pub fn registers_slice_mut(
    &mut self,
    offset: usize,
    sz: usize,
  ) -> &'static mut [LTerm] {
    unsafe { slice::from_raw_parts_mut(self.regs.as_mut_ptr().add(offset), sz) }
  }

  /// Fetch a word from code, assume it is either an `LTerm` or a source X, Y or
  /// FP register, then perform a load operation.
  #[inline]
  pub fn fetch_and_load(&mut self, hp: &heap::Heap) -> LTerm {
    let src = LTerm::from_raw(self.fetch());
    self.load(src, hp)
  }

  //  /// Advance `self.ip` by `n` words.
  //  pub fn ip_add(&mut self, n: isize) {
  //    let CodePtr::Ptr(ip0) = self.ip;
  //    self.ip = unsafe { CodePtr::Ptr(ip0.offset(n)) };
  //  }

  /// Read a register otherwise term is returned unchanged.
  // TODO: Optimize - separate load constant from load register instruction
  pub fn load(&self, src: LTerm, hp: &heap::Heap) -> LTerm {
    if src.is_special() {
      match src.get_special_tag() {
        SPECIALTAG_REGX => return self.get_x(src.get_special_value()),
        SPECIALTAG_REGY => {
          let y_index = src.get_special_value();
          let y_result = hp.get_y(y_index);
          return y_result.unwrap();
        }
        SPECIALTAG_REGFP => panic!("todo fpreg load"),
        _ => return src,
      }
    }
    // Otherwise return unchanged
    src
  }

  /// Copy a value from `src` (possibly a stack cell or a register) to `dst`.
  /// Returns void `()` or an error.
  #[allow(dead_code)]
  #[inline]
  pub fn store_src(
    &mut self,
    src: LTerm,
    dst: LTerm,
    hp: &mut heap::Heap,
  ) -> RtResult<()> {
    let src_val = self.load(src, hp);
    self.store_value(src_val, dst, hp)
  }

  /// Copy a value `val` to `dst`. No attempt is done to load val from a
  /// stack value or a register, val is assumed to be a ready value, not a source.
  /// Returns void `()` or an error.
  pub fn store_value(
    &mut self,
    val: LTerm,
    dst: LTerm,
    hp: &mut heap::Heap,
  ) -> RtResult<()> {
    debug_assert!(
      !val.is_regx(),
      "When storing a value, element must not be regx, have {}",
      val
    );
    debug_assert!(
      !val.is_regy(),
      "When storing a value, element must not be regy, have {}",
      val
    );
    debug_assert!(
      !val.is_regfp(),
      "When storing a value, element must not be regfp, have {}",
      val
    );
    if dst.get_term_tag() == TERMTAG_SPECIAL {
      match dst.get_special_tag() {
        SPECIALTAG_REGX => {
          self.set_x(dst.get_special_value(), val);
          return Ok(());
        }
        SPECIALTAG_REGY => {
          let y = dst.get_special_value();
          return hp.set_y(y, val);
        }
        SPECIALTAG_REGFP => panic!("todo fpreg store"),
        SpecialTag(st) => panic!("store: specialtag {} not supported", st),
      }
    }
    panic!("{}Don't know how to ctx.store {} to {}", module(), val, dst)
  }

  #[inline]
  pub fn set_cp(&mut self, cp: LTerm) {
    debug_assert!(cp.is_cp());
    self.cp = CodePtr::from_cp(cp);
  }

  #[inline]
  pub fn clear_cp(&mut self) {
    self.cp = CodePtr::null();
  }

  #[inline]
  pub fn jump(&mut self, cp: LTerm) {
    debug_assert!(cp.is_cp());
    println!("{} {:p}", "jump to".purple(), cp.get_cp_ptr::<Word>());
    self.ip = CodePtr::from_cp(cp);
  }

  #[inline]
  pub fn jump_ptr(&mut self, code_ptr: *const Word) {
    if cfg!(feature = "trace_opcode_execution") {
      println!("{} {:p}", "jump to".purple(), code_ptr);
    }
    self.ip = CodePtr::from_ptr(code_ptr);
  }

  /// Perform a call to lookup result (done by CodeServer).
  /// Optional `save_cp` defines whether CP will be saved
  pub fn call_mfa(
    &mut self,
    vm: &mut VM,
    curr_p: &mut Process,
    lr: &MFALookupResult,
    args: &[LTerm],
    save_cp: bool,
  ) -> RtResult<()> {
    match lr {
      MFALookupResult::FoundBeamCode(code_p) => {
        if save_cp {
          self.cp = self.ip;
        }
        self.ip = code_p.clone();
      }
      MFALookupResult::FoundBif(bif_fn) => {
        let x0 = call_bif::call_bif_fn(vm, self, curr_p, *bif_fn, args)?;
        self.set_x(0, x0);
      }
    }
    Ok(())
  }
}

// === === === ===
//

impl fmt::Display for Context {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut str_regs = String::new();
    for v in self.regs[0..self.live].iter() {
      str_regs += &format!("{}; ", v)
    }

    writeln!(
      f,
      concat!("Emulator state:\n", "ip: {:?}, cp: {:?}\nregs[..10]: {}"),
      self.ip, self.cp, str_regs
    )
  }
}
