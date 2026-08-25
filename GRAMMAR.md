# Alum Grammar (EBNF)

```
(* ===== Lexical ===== *)

identifier   = ( letter | "_" ) , { letter | digit | "_" } ;
int_lit      = digit , { digit } ;
float_lit    = digit , { digit } , "." , digit , { digit } ;
string_lit   = '"' , { ?any character? | escape } , '"' ;
fstring_lit  = 'f"' , { ?any character? | escape | "{" , expr , "}" } , '"' ;
bool_lit     = "true" | "false" ;
nil_lit      = "nil" ;

keyword      = "fun" | "var" | "cst" | "struct" | "union" | "enum" | "typedef"
             | "if" | "else" | "while" | "for" | "in" | "match" | "return"
             | "break" | "continue" | "import" | "using" | "extern"
             | "true" | "false" | "nil"
             | "int" | "float" | "bool" | "string" | "void" ;

(* Newlines and semicolons carry no syntactic meaning;
   ";" is merely tolerated as an optional separator *)
(* A single "&"/"|" is a logical operator, identical to "&&"/"||";
   bitwise operators are limited to "^" "~" "<<" ">>" *)

(* ===== Preprocessor (standalone text layer) ===== *)

pp_line      = pp_include | pp_define | pp_ifdef | pp_ifndef | pp_else | pp_endif ;
pp_include   = "#include" , file_path ;
pp_define    = "#define" , identifier , [ "(" , id_list , ")" ] , replacement ;
pp_ifdef     = "#ifdef" , identifier ;
pp_ifndef    = "#ifndef" , identifier ;
pp_else      = "#else" ;
pp_endif     = "#endif" ;

(* ===== Program ===== *)

program      = { top_item } ;
top_item     = fun_def | struct_def | union_def | enum_def
             | global_var | const_def | extern_var | typedef_def
             | expr ;

(* ===== Declarations ===== *)

fun_def      = "fun" , [ "(" , fn_anns , ")" ] , identifier ,
               [ "<" , id_list , ">" ] ,
               "(" , params , ")" ,
               [ ":" , type ] ,
               block ;
fn_anns      = fn_ann , { "," , fn_ann } ;
fn_ann       = "pub" | "extern" | "pure" ;
params       = [ param , { "," , param } ] ;
param        = identifier , ":" , type ;

struct_def   = [ "(" , "pub" , ")" ] , "struct" , identifier ,
               [ "<" , id_list , ">" ] , "{" , field , { "," , field } , "}" ;
union_def    = [ "(" , "pub" , ")" ] , "union" , identifier ,
               [ "<" , id_list , ">" ] , "{" , field , { "," , field } , "}" ;
field        = identifier , ":" , type
             | identifier , ":" , type , "(" , params , ")" ;

enum_def     = [ "(" , "pub" , ")" ] , "enum" , identifier ,
               "{" , enum_member , { "," , enum_member } , "}" ;
enum_member  = identifier , [ "=" , int_lit ] ;

global_var   = [ "(" , "pub" , ")" ] , "var" , identifier , [ ":" , type ] , [ "=" , expr ] ;
const_def    = [ "(" , "pub" , ")" ] , "cst" , identifier , [ ":" , type ] , "=" , expr ;
extern_var   = "extern" , "var" , identifier , ":" , type ;
typedef_def  = "typedef" , identifier , "=" , type ;

id_list      = identifier , { "," , identifier } ;

(* ===== Expressions ===== *)

block        = "{" , [ expr , { [ ";" ] , expr } ] , "}" ;

expr         = logical ;
logical      = logical_and , { ( "|" | "||" ) , logical_and } ;
logical_and  = bitwise , { ( "&" | "&&" ) , bitwise } ;
bitwise      = shift , { "^" , shift } ;
shift        = comparison , { ( "<<" | ">>" ) , comparison } ;
comparison   = call , { ( "==" | "!=" | "<" | "<=" | ">" | ">=" ) , call } ;
call         = additive , postfix , { postfix } ;
additive     = term , [ ".." , additive ] , { ( "+" | "-" ) , term } ;
term         = prefix , { ( "*" | "/" | "%" ) , prefix } ;
prefix       = { unary_op } , factor ;
unary_op     = "-" | "!" | "~" | "*" | "&" | "++" | "--" | "$" ;   (* $expr = deep copy *)

postfix      = "." , identifier
             | "[" , expr , "]"
             | "(" , [ args ] , ")"
             | "@" , type ;
args         = expr , { "," , expr } ;

assign_op    = "=" | "+=" | "-=" | "*=" | "/=" | "%="
             | "&=" | "|=" | "^=" | "<<=" | ">>=" ;
inc_dec      = "++" | "--" ;

factor       = int_lit | float_lit | bool_lit | nil_lit | string_lit | fstring_lit
             | "(" , expr , ")"
             | lambda
             | array_literal | array_fill
             | aggregate_literal
             | [ module_path , "::" ] , identifier ,
               [ generic_args ] , [ "(" , [ args ] , ")" ] ;

lambda       = "\" , "(" , params , ")" , ":" , type , block ;
array_literal = "[" , [ expr , { "," , expr } ] , "]" ;
array_fill   = "[" , type , ";" , expr , "]" ;
aggregate_literal = path , [ "<" , type_args , ">" ] , "{" ,
                    [ field_init , { "," , field_init } ] , "}" ;
field_init   = identifier , ":" , expr ;
module_path  = identifier , { "::" , identifier } ;
generic_args = type , { "," , type } ;

if_expr      = "if" , expr , expr , [ "else" , expr ] ;
while_expr   = "while" , expr , expr ;
for_expr     = "for" , identifier , "in" , expr , expr ;
match_expr   = "match" , expr , "{" , match_arm , { match_arm } , [ default_arm ] , "}" ;
match_arm    = expr , ":" , expr ;
default_arm  = "_" , ":" , expr ;
return_expr  = "return" , [ expr ] ;
break_stmt   = "break" ;
continue_stmt = "continue" ;

(* ===== Types ===== *)

type         = "*" , type
             | [ module_path , "::" ] , named_type ;
named_type   = prim_type
             | identifier , [ "<" , type_args , ">" ] ;
prim_type    = "int" | "float" | "bool" | "string" | "void" ;
type_args    = type , { "," , type } ;

(* ===== Modules ===== *)

import_stmt  = "import" , ( string_lit | module_path ) ;
using_stmt   = "using" , using_target ;
using_target = module_path
             | module_path , "::" , "{" , name_list , "}"
             | module_path , "::" , identifier , [ "as" , identifier ] ;
name_list    = identifier , { "," , identifier } ;
```
