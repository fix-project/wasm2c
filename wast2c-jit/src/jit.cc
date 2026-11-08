// LLVM-based JIT for the wasm2c generated code. Partly written by Claude Opus 5.5.

#include "clang/Basic/Diagnostic.h"
#include "clang/Basic/DiagnosticOptions.h"
#include "clang/CodeGen/CodeGenAction.h"
#include "clang/Driver/CreateInvocationFromArgs.h"
#include "clang/Frontend/CompilerInstance.h"
#include "clang/Frontend/CompilerInvocation.h"
#include "clang/Frontend/FrontendActions.h"
#include "clang/Frontend/FrontendOptions.h"
#include "clang/Lex/PreprocessorOptions.h"
#include "clang/Options/OptionUtils.h"
#include "llvm/ADT/IntrusiveRefCntPtr.h"
#include "llvm/ADT/SmallVector.h"
#include "llvm/ADT/StringRef.h"
#include "llvm/ExecutionEngine/Orc/Core.h"
#include "llvm/ExecutionEngine/Orc/JITTargetMachineBuilder.h"
#include "llvm/ExecutionEngine/Orc/LLJIT.h"
#include "llvm/ExecutionEngine/Orc/Shared/ExecutorAddress.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/Support/CodeGen.h"
#include "llvm/Support/Error.h"
#include "llvm/Support/MemoryBuffer.h"
#include "llvm/Support/SmallVectorMemoryBuffer.h"
#include "llvm/Support/TargetSelect.h"
#include "llvm/Support/VirtualFileSystem.h"
#include "llvm/Support/VirtualOutputBackend.h"
#include "llvm/Support/VirtualOutputConfig.h"
#include "llvm/Support/VirtualOutputFile.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/TargetParser/Host.h"
#include <array>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <exception>
#include <iostream>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>

#ifndef CLANG_PATH
#error "CLANG_PATH must be defined by build.rs"
#endif

#ifndef CC_WARNINGS
#error "CC_WARNINGS must be defined by build.rs"
#endif

using namespace llvm;

namespace {

constexpr const char* kSourcePath = "/input.cc";
constexpr const char* kHeaderPath = "/input.hh";
constexpr const char* kPreludePath = "/prelude.hh";
constexpr const char* kPchPath = "/prelude.pch";

IntrusiveRefCntPtr<vfs::FileSystem> overlay_on_real_fs( IntrusiveRefCntPtr<vfs::InMemoryFileSystem> in_memory )
{
  auto overlay = makeIntrusiveRefCnt<vfs::OverlayFileSystem>( vfs::getRealFileSystem() );
  overlay->pushOverlay( std::move( in_memory ) );
  return overlay;
}

template<class T>
class NotNull // (until moved-from)
{
  std::unique_ptr<T> internal_;

public:
  template<typename... Targs>
  explicit NotNull( const std::string_view context, Targs&&... Fargs )
    : internal_( std::forward<Targs>( Fargs )... )
  {
    if ( !internal_ ) {
      throw std::runtime_error( std::string( context ) + ": returned null pointer" );
    }
  }

  T* operator->() { return internal_.get(); }
  const T* operator->() const { return internal_.get(); }
  operator std::unique_ptr<T>() && { return std::move( internal_ ); } // NOLINT(*-explicit-*)
};

NotNull<clang::CompilerInvocation> make_compiler_invocation( const char* input_path, const char* language )
{
  static const std::string target = "--target=" + sys::getProcessTriple();
  static const std::string resource_dir = clang::GetResourcesPath( CLANG_PATH );

  const std::array args = {
    CLANG_PATH,
    target.c_str(),
    "-std=c++23",
    "-fPIC",
    "-fpch-instantiate-templates",
    "-resource-dir",
    resource_dir.c_str(),
    CC_WARNINGS,
    // use strict C++ warnings, but turn off some for the generated code:
    "-Wno-unused-variable",
    "-Wno-unused-parameter",
    "-Wno-unused-label",
    "-Wno-unused-private-field",
    "-fsyntax-only", // FrontendAction decides what actually happens later
    "-x",
    language,
    input_path,
  };

  NotNull<clang::CompilerInvocation> ret { "clang::createInvocation", clang::createInvocation( args ) };
  ret->getFrontendOpts().DisableFree = false;
  ret->getCodeGenOpts().DisableFree = false;
  return ret;
}

void execute_action( std::unique_ptr<clang::CompilerInvocation> invocation,
                     IntrusiveRefCntPtr<vfs::FileSystem> vfs,
                     clang::FrontendAction& action,
                     IntrusiveRefCntPtr<vfs::OutputBackend> outputs )
{
  clang::CompilerInstance compiler( std::shared_ptr<clang::CompilerInvocation>( std::move( invocation ) ) );
  compiler.setVirtualFileSystem( std::move( vfs ) );
  compiler.setOutputManager( std::move( outputs ) );
  compiler.createDiagnostics();

  if ( !compiler.ExecuteAction( action ) ) {
    throw std::runtime_error( "compiler action failed" );
  }
}

class MemoryOutputFile : public vfs::OutputFileImpl
{
  std::string path_;
  std::unique_ptr<MemoryBuffer>* destination_;
  SmallVector<char, 0> contents_;
  raw_svector_ostream stream_ { contents_ };

public:
  MemoryOutputFile( StringRef path, std::unique_ptr<MemoryBuffer>& destination )
    : path_( path.str() ), destination_( &destination )
  {}

  Error keep() override
  {
    *destination_ = std::make_unique<SmallVectorMemoryBuffer>(
      std::move( contents_ ), path_, /*RequiresNullTerminator=*/false );
    return Error::success();
  }
  Error discard() override { return Error::success(); }
  raw_pwrite_stream& getOS() override { return stream_; }
};

class ForbidOutputBackend : public vfs::OutputBackend
{
protected:
  Expected<std::unique_ptr<vfs::OutputFileImpl>> createFileImpl( StringRef path,
                                                                 std::optional<vfs::OutputConfig> config
                                                                 [[maybe_unused]] ) override
  { return createStringError( inconvertibleErrorCode(), "forbidden attempt to create file: " + path.str() ); }

  IntrusiveRefCntPtr<vfs::OutputBackend> cloneImpl() const override
  { return makeIntrusiveRefCnt<ForbidOutputBackend>(); }
};

class MemoryOutputBackend : public vfs::OutputBackend
{
  std::string path_;
  std::unique_ptr<MemoryBuffer>* destination_;

protected:
  IntrusiveRefCntPtr<vfs::OutputBackend> cloneImpl() const override
  { return makeIntrusiveRefCnt<MemoryOutputBackend>( path_, *destination_ ); }

  Expected<std::unique_ptr<vfs::OutputFileImpl>> createFileImpl( StringRef path,
                                                                 std::optional<vfs::OutputConfig> config
                                                                 [[maybe_unused]] ) override
  {
    if ( path != path_ ) {
      return createStringError( inconvertibleErrorCode(), "attempt to write to incorrect path: " + path.str() );
    }
    return std::make_unique<MemoryOutputFile>( path, *destination_ );
  }

public:
  MemoryOutputBackend( StringRef path, std::unique_ptr<MemoryBuffer>& destination )
    : path_( path.str() ), destination_( &destination )
  {}
};

template<typename T>
T llvm_do( Expected<T> result )
{
  if ( !result ) {
    throw std::runtime_error( "LLVM error: " + toString( result.takeError() ) );
  }
  return std::move( *result );
}

void llvm_catch( Error maybe_error )
{
  if ( maybe_error ) {
    throw std::runtime_error( "LLVM error: " + toString( std::move( maybe_error ) ) );
  }
}

class PreCompiledHeaders
{
  std::string includes_;
  std::unique_ptr<MemoryBuffer> pch_;

public:
  explicit PreCompiledHeaders( std::string includes ) : includes_( std::move( includes ) )
  {
    auto vfs = makeIntrusiveRefCnt<vfs::InMemoryFileSystem>();
    vfs->addFile( kPreludePath, 0, MemoryBuffer::getMemBuffer( includes_ ) );
    auto invocation = make_compiler_invocation( kPreludePath, "c++-header" );
    invocation->getFrontendOpts().OutputFile = kPchPath;
    clang::GeneratePCHAction action;
    execute_action( std::move( invocation ),
                    overlay_on_real_fs( vfs ),
                    action,
                    makeIntrusiveRefCnt<MemoryOutputBackend>( kPchPath, pch_ ) );
    if ( !pch_ ) {
      throw std::runtime_error( "precompiled header not written" );
    }
    if ( pch_->getBufferSize() == 0 ) {
      throw std::runtime_error( "precompiled header is empty" );
    }
  }

  [[nodiscard]] const std::string& includes() const { return includes_; }
  [[nodiscard]] std::unique_ptr<MemoryBuffer> dup() const
  { return MemoryBuffer::getMemBuffer( pch_->getMemBufferRef(), false ); }
};

// RAII wrapper for JIT engine
class Runnable
{
  NotNull<orc::LLJIT> engine_;
  orc::JITDylib& library_ { engine_->getMainJITDylib() };

  static NotNull<Module> compile( StringRef header,
                                  LLVMContext& context,
                                  StringRef source,
                                  const PreCompiledHeaders& pch )
  {
    auto vfs = makeIntrusiveRefCnt<vfs::InMemoryFileSystem>();
    vfs->addFile( kSourcePath, 0, MemoryBuffer::getMemBuffer( source ) );
    vfs->addFile( kHeaderPath, 0, MemoryBuffer::getMemBuffer( header ) );
    vfs->addFile( kPreludePath, 0, MemoryBuffer::getMemBuffer( pch.includes() ) );
    vfs->addFile( kPchPath, 0, pch.dup() );
    auto invocation = make_compiler_invocation( kSourcePath, "c++" );
    invocation->getPreprocessorOpts().ImplicitPCHInclude = kPchPath;
    clang::EmitLLVMOnlyAction action( &context );
    execute_action( std::move( invocation ),
                    overlay_on_real_fs( std::move( vfs ) ),
                    action,
                    makeIntrusiveRefCnt<ForbidOutputBackend>() );
    NotNull<Module> ret { "compile", action.takeModule() };
    return ret;
  }

  static NotNull<orc::LLJIT> make_engine()
  {
    orc::JITTargetMachineBuilder machine = llvm_do( orc::JITTargetMachineBuilder::detectHost() );
    machine.setCodeGenOptLevel( CodeGenOptLevel::None );
    return NotNull<orc::LLJIT> {
      "LLJITBuilder", llvm_do( orc::LLJITBuilder().setJITTargetMachineBuilder( std::move( machine ) ).create() ) };
  }

public:
  Runnable( StringRef header, StringRef source, const PreCompiledHeaders& pch ) : engine_( make_engine() )
  {
    auto context = std::make_unique<LLVMContext>();
    auto module = compile( header, *context, source, pch );
    llvm_catch( engine_->addIRModule( orc::ThreadSafeModule( std::move( module ), std::move( context ) ) ) );
    llvm_catch( engine_->initialize( library_ ) );
  }

  // Runs a function and passes along its return value.
  template<typename T>
  T execute( const std::string_view function_name )
  {
    const orc::ExecutorAddr run_address = llvm_do( engine_->lookup( function_name ) );
    return run_address.toPtr<T ( * )()>()();
  }

  ~Runnable()
  {
    try {
      llvm_catch( engine_->deinitialize( library_ ) );
    } catch ( const std::exception& e ) {
      std::cerr << "Exception in Runnable destructor: " << e.what() << "\n";
    }
  }

  Runnable( const Runnable& ) = delete;
  Runnable& operator=( const Runnable& ) = delete;
  Runnable( Runnable&& ) = delete;
  Runnable& operator=( Runnable&& ) = delete;
};

std::optional<PreCompiledHeaders>& global_pch()
{
  static std::optional<PreCompiledHeaders> pch;
  return pch;
}
} // namespace

// Must be called *once* before any `run_program` calls. Not thread-safe.
extern "C" int init_with_precompiled_headers( const char* includes, size_t includes_length ) noexcept
{
  const bool initialized
    = !InitializeNativeTarget() && !InitializeNativeTargetAsmPrinter() && !InitializeNativeTargetAsmParser();
  if ( !initialized ) {
    std::cerr << "cannot initialize the native LLVM target\n";
    return 1;
  }

  auto& pch = global_pch();
  if ( pch ) {
    std::cerr << "global precompiled headers already made\n";
    return 1;
  }

  try {
    pch.emplace( std::string { includes, includes_length } );
    return 0;
  } catch ( const std::exception& e ) {
    std::cerr << "Exception: " << e.what() << "\n";
    return 1;
  }
}

// Compiles and runs generated C++ whose `extern "C" unsigned run()` returns
// the number of successful Wast directives/commands. Returns 0 on success.
extern "C" int run_program( const char* header,
                            size_t header_length,
                            const char* source,
                            size_t source_length,
                            uint32_t* num_successes ) noexcept
{
  const auto& pch = global_pch();
  if ( !pch ) {
    std::cerr << "precompiled headers not yet precompiled\n";
    return 1;
  }

  const StringRef hh_ref { header, header_length };
  const StringRef cc_ref { source, source_length };

  if ( hh_ref.empty() or cc_ref.empty() or hh_ref.back() != 0 or cc_ref.back() != 0 ) {
    std::cerr << "input not NUL-terminated\n";
    return 1;
  }

  try {
    Runnable module { hh_ref.drop_back( 1 ), cc_ref.drop_back( 1 ), *pch };
    *num_successes = module.execute<unsigned int>( "run" );
    return 0;
  } catch ( const std::exception& e ) {
    std::cerr << "Exception: " << e.what() << "\n";
    return 1;
  } catch ( ... ) {
    std::cerr << "Exception of unknown type.\n";
    return 1;
  }
}
