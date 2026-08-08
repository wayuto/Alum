export codes=$(find . -type f -regextype posix-extended -regex ".*\.(al|c|cpp)$")
export compile_al="alc factorial.al -o factorial_al"
export compile_cpp="g++ factorial.cpp -o factorial_cpp -O3 -fpermissive"

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

echo "========== Result =========="
ls -l factorial_*
echo

echo "========== Running =========="
echo "Alum: $(./factorial_al)"
echo "C++: $(./factorial_cpp)"
echo

echo "========== Benchmark =========="

hyperfine "./factorial_cpp" "./factorial_al" --warmup 10 -i --shell none
rm factorial_*
