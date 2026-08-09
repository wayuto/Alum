export codes=$(find . -type f -regextype posix-extended -regex ".*\.(al|c|cpp|rs|zig|py)$")
export compile_al="alc factorial.al -o factorial_al"
export compile_cpp="g++ factorial.cpp -o factorial_cpp -O3 -fconstexpr-ops-limit=210000000"
export compile_rust="rustc -O factorial.rs -o factorial_rust"
export compile_zig="zig build-exe factorial.zig -O ReleaseFast -femit-bin=factorial_zig"

for code in $codes
do
        echo
        echo "========= $code ==========="
        cat $code
        echo
done

echo "========== $compile_al =========="
time bash -c "$compile_al"
echo
echo "==========  $compile_cpp  =========="
time bash -c "$compile_cpp"
echo
echo "==========  $compile_rust  =========="
time bash -c "$compile_rust"
echo
echo "==========  $compile_zig  =========="
time bash -c "$compile_zig"
echo
echo "========== Result =========="
ls -l factorial_*
echo

echo "========== Running =========="
echo "Alum: $(./factorial_al)"
echo "C++: $(./factorial_cpp)"
echo "Rust: $(./factorial_rust)"
echo "Zig: $(./factorial_zig)"
echo "Python: $(python3 factorial.py)"
echo

echo "========== Benchmark =========="

hyperfine "./factorial_cpp" "./factorial_al" "./factorial_rust" "./factorial_zig" "python3 factorial.py" --warmup 10 -m 5 -i --shell none
rm factorial_*