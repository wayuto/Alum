gos -c foo.gos
gcc -O3 foo.c
hyperfine -i './foo' './a.out' 'python foo.py' --shell=none --warmup 100
rm foo a.out