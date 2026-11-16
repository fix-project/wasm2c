// The host: times CALLS calls of each export, ROUNDS times, taking turns.
// Prints the fastest and the median round, in nanoseconds per call.
//
//   ./entry_cost            all three variants (the benchmark)
//   ./entry_cost setjmp     one variant only (so `perf stat` can count its cycles)
#include <algorithm>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <vector>

// Defined in exports.cc. Each returns x + 1.
int export_plain( int x );
int export_unwind( int x );
int export_setjmp( int x );

constexpr int CALLS = 20'000'000;
constexpr int ROUNDS = 25; // plus one warm-up round that is not recorded

// The timed loop, one copy per export so that the call is a direct call.
// Each call's argument is the previous call's result, so the CPU cannot
// overlap calls and the compiler cannot drop any.
template<int ( *EXPORT )( int )>
[[gnu::noinline]] int call_repeatedly()
{
  int x = 0;
  for ( int i = 0; i < CALLS; i++ ) {
    x = EXPORT( x );
  }
  return x;
}

struct variant
{
  const char* name;
  int ( *loop )();
  std::vector<double> ns_per_call; // one entry per round
};

int main( int argc, char** argv )
{
  std::vector<variant> variants = {
    { "plain", call_repeatedly<export_plain>, {} },
    { "unwind", call_repeatedly<export_unwind>, {} },
    { "setjmp", call_repeatedly<export_setjmp>, {} },
  };
  if ( argc > 1 ) {
    std::erase_if( variants, [&]( const variant& v ) { return std::strcmp( v.name, argv[1] ) != 0; } );
  }

  for ( int round = -1; round < ROUNDS; round++ ) { // round -1 is the warm-up
    for ( variant& v : variants ) {
      const auto start = std::chrono::steady_clock::now();
      const int result = v.loop();
      const std::chrono::duration<double, std::nano> elapsed = std::chrono::steady_clock::now() - start;

      if ( result != CALLS ) { // every call really happened
        std::fprintf( stderr, "%s: wrong result %d\n", v.name, result );
        return 1;
      }
      if ( round >= 0 ) {
        v.ns_per_call.push_back( elapsed.count() / CALLS );
      }
    }
  }

  std::printf( "%-8s %8s %8s   ns per call\n", "variant", "min", "median" );
  for ( variant& v : variants ) {
    std::sort( v.ns_per_call.begin(), v.ns_per_call.end() );
    std::printf( "%-8s %8.2f %8.2f\n", v.name, v.ns_per_call.front(), v.ns_per_call[ROUNDS / 2] );
  }
}
