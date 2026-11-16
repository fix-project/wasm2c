// Three versions of the same exported wasm function, x -> x + 1. They differ
// only in what happens on entry, which is what wasm_entry in ../src/runtime.hh
// does in each of its three configurations.
//
// This file is compiled separately from main.cc and without LTO, so the host
// loop cannot see into these functions, as with real generated code.
#include <setjmp.h>
#include <stdexcept>
#include <utility>

// 1. plain: nothing on entry. What a bounds-checked build does
//    (W2CC_GUARD_PAGES=0).

int export_plain( int x )
{
  return x + 1;
}

// 2. unwind: a wrapper that is never inlined. What a build does where the
//    fault handler throws (W2CC_GUARD_PAGES=1, W2CC_TRAP_UNWIND=1).

template<typename F>
[[gnu::noinline]] auto entry_unwind( F&& f ) -> decltype( f() )
{
#ifdef __clang__
  asm volatile( "" ::: "unwind" ); // emits no instructions
#endif
  return std::forward<F>( f )();
}

int export_unwind( int x )
{
  return entry_unwind( [&] { return x + 1; } );
}

// 3. setjmp: sigsetjmp, plus remembering the jump target in a thread-local.
//    What a build does where the fault handler jumps back to the export
//    (W2CC_GUARD_PAGES=1, W2CC_TRAP_UNWIND=0).

struct trap_target
{
  sigjmp_buf env;
  volatile int kind;
};

thread_local constinit trap_target* active_target = nullptr;

struct trap_target_scope
{
  explicit trap_target_scope( trap_target* target ) noexcept : saved( std::exchange( active_target, target ) ) {}
  ~trap_target_scope() { active_target = saved; }
  trap_target* saved;
};

template<typename F>
auto entry_setjmp( F&& f ) -> decltype( f() )
{
  trap_target target;
  trap_target_scope scope { &target };
  if ( sigsetjmp( target.env, 0 ) != 0 ) [[unlikely]] {
    throw std::runtime_error( "trap" ); // never happens here: nothing traps
  }
  return std::forward<F>( f )();
}

int export_setjmp( int x )
{
  return entry_setjmp( [&] { return x + 1; } );
}
