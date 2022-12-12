#!/usr/bin/env sh
# This script expects a path to a local nand to tetris installation
# as its first argument.
# The CPU test will only work, if the asm files in project 4 have been implemented.
# All the other tests should work on a fresh download.

cargo test

# VM
cargo run --bin=vm -- "$1/projects/07/MemoryAccess/BasicTest/BasicTestVME.tst"
cargo run --bin=vm -- "$1/projects/07/MemoryAccess/PointerTest/PointerTestVME.tst"
cargo run --bin=vm -- "$1/projects/07/MemoryAccess/StaticTest/StaticTestVME.tst"

cargo run --bin=vm -- "$1/projects/08/FunctionCalls/FibonacciElement/FibonacciElementVME.tst"
cargo run --bin=vm -- "$1/projects/08/FunctionCalls/NestedCall/NestedCallVME.tst"
cargo run --bin=vm -- "$1/projects/08/FunctionCalls/SimpleFunction/SimpleFunctionVME.tst"
cargo run --bin=vm -- "$1/projects/08/FunctionCalls/StaticsTest/StaticsTestVME.tst"
cargo run --bin=vm -- "$1/projects/08/ProgramFlow/BasicLoop/BasicLoopVME.tst"
cargo run --bin=vm -- "$1/projects/08/ProgramFlow/FibonacciSeries/FibonacciSeriesVME.tst"

# VM (with compilation step)
/usr/bin/env sh "$1/tools/JackCompiler.sh" "$1/projects/12/ArrayTest"
cargo run --bin=vm -- "$1/projects/12/ArrayTest/ArrayTest.tst"
/usr/bin/env sh "$1/tools/JackCompiler.sh" "$1/projects/12/MathTest"
cargo run --bin=vm -- "$1/projects/12/MathTest/MathTest.tst"
/usr/bin/env sh "$1/tools/JackCompiler.sh" "$1/projects/12/MemoryTest"
 cargo run --bin=vm -- "$1/projects/12/MemoryTest/MemoryTest.tst"
/usr/bin/env sh "$1/tools/JackCompiler.sh" "$1/projects/12/MemoryTest/MemoryDiag"
 cargo run --bin=vm -- "$1/projects/12/MemoryTest/MemoryDiag/MemoryDiag.tst"

# CPU
cargo run --bin=cpu -- "$1/projects/04/fill/FillAutomatic.tst"
cargo run --bin=cpu -- "$1/projects/04/mult/Mult.tst"
