// Can be replaced by compiling with -DW2CC_RUNTIME="custom_runtime.hh"
#ifndef W2CC_RUNTIME_HH
#define W2CC_RUNTIME_HH

#ifdef W2CC_RUNTIME
#include W2CC_RUNTIME
#else

#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <new>
#include <span>
#include <stdexcept>

namespace w2cc {
struct trap_error : std::runtime_error
{
  using std::runtime_error::runtime_error;
};

// wasm_rt_trap. TODO: use longjmp instead of C++ exceptions
[[noreturn]] inline void trap( const char* reason )
{ throw trap_error( reason ); }

// Only bounds-checked. TODO: use guard pages
class memory
{
public:
  static constexpr uint64_t page_size = 65536;

  memory( uint64_t initial_pages, uint64_t max_pages ) : size_( initial_pages * page_size ), max_pages_( max_pages )
  {
    data_ = static_cast<uint8_t*>( std::calloc( size_, 1 ) );
    if ( data_ == nullptr && size_ != 0 ) {
      throw std::bad_alloc();
    }
  }

  ~memory() { std::free( data_ ); }

  // Prevent copying
  memory( const memory& ) = delete;
  memory& operator=( const memory& ) = delete;

  // memory.size
  uint32_t pages() const { return static_cast<uint32_t>( size_ / page_size ); }

  // memory.grow
  uint32_t grow( uint32_t delta )
  {
    uint32_t old_pages = pages();
    uint64_t new_pages = uint64_t { old_pages } + delta;
    if ( new_pages > max_pages_ ) {
      return static_cast<uint32_t>( -1 );
    }
    if ( delta ) {
      uint64_t new_size = new_pages * page_size;
      auto* new_data = static_cast<uint8_t*>( std::realloc( data_, new_size ) );
      if ( new_data == nullptr ) {
        return static_cast<uint32_t>( -1 );
      }
      std::memset( new_data + size_, 0, new_size - size_ );
      data_ = new_data;
      size_ = new_size;
    }
    return old_pages;
  }

  // template over all load operations
  template<typename T>
  T load( uint32_t address, uint64_t offset ) const
  {
    T value;
    std::memcpy( &value, data_ + checked( address, offset, sizeof( T ) ), sizeof( T ) );
    return value;
  }

  // template over all store operations
  template<typename T>
  void store( uint32_t address, uint64_t offset, T value )
  { std::memcpy( data_ + checked( address, offset, sizeof( T ) ), &value, sizeof( T ) ); }

  // copies data segment into memory
  void init( uint32_t offset, std::span<const uint8_t> data )
  {
    uint64_t start = checked( offset, 0, data.size() );
    if ( !data.empty() ) {
      std::memcpy( data_ + start, data.data(), data.size() );
    }
  }

private:
  uint64_t checked( uint32_t address, uint64_t offset, uint64_t size ) const
  {
    uint64_t start = uint64_t { address } + offset;
    if ( start + size > size_ ) [[unlikely]] {
      trap( "out of bounds memory access" );
    }
    return start;
  }

  uint8_t* data_;
  uint64_t size_;
  uint64_t max_pages_;
};

}

#endif // W2CC_RUNTIME
#endif // W2CC_RUNTIME_HH
