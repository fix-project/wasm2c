#include "clang/Basic/Diagnostic.h"
#include "clang/Basic/DiagnosticIDs.h"
#include "clang/Basic/DiagnosticOptions.h"
#include "clang/CodeGen/CodeGenAction.h"
#include "clang/Driver/CreateInvocationFromArgs.h" // clang::createInvocation
#include "clang/Frontend/CompilerInstance.h"
#include "clang/Frontend/CompilerInvocation.h"
#include "clang/Frontend/TextDiagnosticPrinter.h"
#include "llvm/ExecutionEngine/Orc/ExecutionUtils.h"
#include "llvm/ExecutionEngine/Orc/LLJIT.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/Support/MemoryBuffer.h"
#include "llvm/Support/TargetSelect.h"
#include "llvm/Support/VirtualFileSystem.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/TargetParser/Host.h"
#include <cstddef>
#include <string>
#include <utility>

#ifndef JIT_CLANG_RESOURCE_DIR
#error "JIT_CLANG_RESOURCE_DIR must be defined by build.rs"
#endif

#ifndef JIT_SYSTEM_INCLUDE_PATHS
#error "JIT_SYSTEM_INCLUDE_PATHS must be defined by build.rs"
#endif

// The clang driver derives both its resource directory (builtin headers such
// as stddef.h) and its GCC toolchain search paths from argv[0]. We are not
// running as clang++, so point it at the real driver binary explicitly.
static constexpr const char *kSystemIncludePaths[] = JIT_SYSTEM_INCLUDE_PATHS;

using namespace llvm;

extern "C" int run_program(const char *source, std::size_t source_length,
                           const char *header, std::size_t header_length,
                           int *result) noexcept {
  // The asm parser is required too: the C++ standard library headers pull in
  // inline asm, which the JIT cannot lower without it.
  if (InitializeNativeTarget() || InitializeNativeTargetAsmPrinter() ||
      InitializeNativeTargetAsmParser()) {
    return 1;
  }

  // Clang 23: DiagnosticOptions is NO LONGER refcounted (no Retain/Release),
  // so it cannot go in an IntrusiveRefCntPtr. Hold it in a shared_ptr and
  // pass it by reference everywhere.
  auto diag_opts = std::make_shared<clang::DiagnosticOptions>();

  // DiagnosticsEngine is still refcounted, but its constructor now takes
  // DiagnosticOptions *by reference*.
  IntrusiveRefCntPtr<clang::DiagnosticsEngine> diags =
      new clang::DiagnosticsEngine(
          new clang::DiagnosticIDs(), *diag_opts,
          new clang::TextDiagnosticPrinter(errs(), *diag_opts),
          /*ShouldOwnClient=*/true);

  std::unique_ptr<LLVMContext> context = std::make_unique<LLVMContext>();
  std::unique_ptr<Module> module;

  // Compile C++ source
  {
    IntrusiveRefCntPtr<vfs::InMemoryFileSystem> in_mem_fs(
        new vfs::InMemoryFileSystem());

    // These basenames are a contract with the Rust side: codegen emits
    // `#include "<Output::name>.hh"`, so the caller's Output::name must be
    // "input". Passing the name through the FFI would make that explicit.
    in_mem_fs->addFile("/input.cc", 0,
                       MemoryBuffer::getMemBufferCopy(
                           StringRef(source, source_length), "/input.cc"));
    in_mem_fs->addFile("/input.hh", 0,
                       MemoryBuffer::getMemBufferCopy(
                           StringRef(header, header_length), "/input.hh"));

    auto overlay =
        makeIntrusiveRefCnt<vfs::OverlayFileSystem>(vfs::getRealFileSystem());
    overlay->pushOverlay(in_mem_fs);

    // Resolve against the installed clang, not /proc/self/exe (which is the
    // Rust binary this library is linked into).
    std::string resource_dir = JIT_CLANG_RESOURCE_DIR;
    const std::string target = sys::getProcessTriple();

    // Driver-level arguments ("clang++ ..."), NOT -cc1 arguments. The driver
    // resolves the C++ standard library include paths for us.
    std::vector<std::string> arg_strings = {
        "clang++", // argv[0]: puts the driver in C++ mode, and anchors its
                   // toolchain detection at the real installation
        "--target=" + target,
        "-std=c++23",
        "-fPIC",
        "-resource-dir",
        resource_dir,
        "-fsyntax-only", // just get a compile job out of the driver;
                         // EmitLLVMOnlyAction below decides what the
                         // frontend actually emits
        "-x",
        "c++",
        "/input.cc",
    };

    for (const char *path : kSystemIncludePaths) {
      arg_strings.emplace_back("-isystem");
      arg_strings.emplace_back(path);
    }

    std::vector<const char *> args;
    args.reserve(arg_strings.size());
    for (const std::string &s : arg_strings) {
      args.push_back(s.c_str());
    }

    clang::CreateInvocationOptions options;
    options.VFS = overlay; // driver sees /input.cc and /input.hh
    options.Diags = diags; // share our diagnostics engine

    std::unique_ptr<clang::CompilerInvocation> invocation =
        clang::createInvocation(args, options);
    if (!invocation) {
      return 1;
    }

    // Clang 23: CompilerInstance owns its invocation as a shared_ptr and takes
    // it via the constructor; there is no setInvocation() anymore.
    clang::CompilerInstance compiler(
        std::shared_ptr<clang::CompilerInvocation>(std::move(invocation)));
    compiler.setVirtualFileSystem(overlay);

    // Build the frontend's engine from the invocation's own diagnostic
    // options rather than reusing `diags`: the hand-made one above never went
    // through ProcessWarningOptions, so it would report every warning from
    // inside the libstdc++ headers. Defaults to printing to stderr.
    compiler.createDiagnostics();

    // Action that executes frontend pipeline without writing to disk
    clang::EmitLLVMOnlyAction action(context.get());
    if (!compiler.ExecuteAction(action)) {
      return 1;
    }
    module = action.takeModule();
    if (!module) {
      return 1;
    }
  }

  // Create execution engine
  Expected<std::unique_ptr<orc::LLJIT>> lljit = orc::LLJITBuilder().create();
  if (!lljit) {
    errs() << toString(lljit.takeError()) << '\n';
    return 1;
  }
  std::unique_ptr<orc::LLJIT> engine = std::move(*lljit);
  Error error = engine->addIRModule(
      orc::ThreadSafeModule(std::move(module), std::move(context)));
  if (error) {
    errs() << toString(std::move(error)) << '\n';
    return 1;
  }

  orc::JITDylib &library = engine->getMainJITDylib();

  // The generated code calls into the C++ runtime (std::cout, operator new,
  // __cxa_*). Those live in this process; let the JIT resolve against them.
  Expected<std::unique_ptr<orc::DynamicLibrarySearchGenerator>> generator =
      orc::DynamicLibrarySearchGenerator::GetForCurrentProcess(
          engine->getDataLayout().getGlobalPrefix());
  if (!generator) {
    errs() << toString(generator.takeError()) << '\n';
    return 1;
  }
  library.addGenerator(std::move(*generator));

  error = engine->initialize(library);
  if (error) {
    errs() << toString(std::move(error)) << '\n';
    return 1;
  }

  Expected<orc::ExecutorAddr> main_address = engine->lookup("main");
  if (!main_address) {
    errs() << toString(main_address.takeError()) << '\n';
    return 1;
  }

  int status = 0;
  try {
    *result = main_address->toPtr<int (*)()>()();
  } catch (...) {
    errs() << "The C++ program threw an exception\n";
    status = 1;
  }

  error = engine->deinitialize(library);
  if (error) {
    errs() << toString(std::move(error)) << '\n';
    return 1;
  }

  return status;
}
