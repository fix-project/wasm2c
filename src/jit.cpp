#include "clang/CodeGen/CodeGenAction.h"
#include "clang/Frontend/CompilerInstance.h"
#include "clang/Frontend/CompilerInvocation.h"
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
#include <memory>
#include <string>
#include <utility>

using namespace llvm;

extern "C" int run_program(const char *source, std::size_t length,
                           int *result) noexcept {
  if (InitializeNativeTarget() || InitializeNativeTargetAsmPrinter()) {
    return 1;
  }
  std::unique_ptr<LLVMContext> context = std::make_unique<LLVMContext>();
  std::unique_ptr<Module> module;

  // Compile C++ source
  {
    IntrusiveRefCntPtr<vfs::InMemoryFileSystem> InMemFS(
        new vfs::InMemoryFileSystem());

    std::unique_ptr<MemoryBuffer> buffer =
        MemoryBuffer::getMemBufferCopy(StringRef(source, length), "/input.cpp");
    InMemFS->addFile("/input.cpp", 0, std::move(buffer));

    clang::CompilerInstance compiler;
    compiler.setVirtualFileSystem(InMemFS);
    compiler.createDiagnostics();

    // Compile for target platform
    const std::string target = sys::getProcessTriple();
    const char *args[] = {"-triple",
                          target.c_str(),
                          "-std=c++23",
                          "-emit-llvm-only",
                          "-mrelocation-model",
                          "pic",
                          "-pic-level",
                          "2",
                          "/input.cpp"};

    if (!clang::CompilerInvocation::CreateFromArgs(
            compiler.getInvocation(), args, compiler.getDiagnostics())) {
      return 1;
    }

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
