al fib1000.al
gcc -O3 fib1000.c
hyperfine -i './fib1000' './a.out' 'python fib1000.py' --shell=none --warmup 100
rm fib1000 a.out