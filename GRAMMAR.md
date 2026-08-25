# Alum Grammar (EBNF)

Reflects the implementation in `src/compiler/` as of Alum 0.9.9.

```
(* ===== Lexical ===== *)

identifier   = ( letter | "_" ) , { letter | digit | "_" } ;
(* letters/digits here follow Unicode is_alphabetic/is_alphanumeric;
   int_lit/float_lit accept ASCII digits only *)

int_lit      = digit , { digit } ;
float_lit    = digit , { digit } , "." , digit , { digit } ,
               [ ( "e" | "E" ) , [ "+" | "-" ] , digit , { digit } ] ;
string_lit   = string ;
string       = ( '"' , { char } , '"' )
             | ( "'" , { char } , "'" ) ;        (* may span lines *)
char         = ?any character? | escape ;
escape       = "\" , ( "n" | "t" | "r" | ?any character? ) ;
fstring_lit  = "f" , quote , { fstring_char } , quote ;   (* quote is '"' or "'" *)
fstring_char = char
             | "{{" | "}}"                        (* escaped braces *)
             | "{" , expr_source , "}" ;          (* interpolated expression, re-lexed *)
bool_lit     = "true" | "false" ;
nil_lit      = "nil" ;

comment      = "//" , { ?any character except newline? } ;

keyword      = "fun" | "var" | "cst" | "struct" | "union" | "enum" | "typedef"
             | "if" | "else" | "while" | "for" | "in" | "match" | "return"
             | "break" | "continue" | "import" | "using" | "as" | "extern"
             | "true" | "false" | "nil"
             | "int" | "float" | "bool" | "string" | "void" ;

(* "pub" and "pure" are ordinary identifiers used contextually, not keywords.
   Newlines carry no syntactic meaning; ";" is tolerated as an optional
   separator inside blocks only (a ";" at top level is a syntax error).
   A single "&"/"|" is a logical operator, identical to "&&"/"||";
   bitwise operators are limited to "^" "~" "<<" ">>". *)

(* ===== Preprocessor (standalone text layer) ===== *)

pp_line      = pp_include | pp_define | pp_ifdef | pp_ifndef | pp_else | pp_endif ;
pp_include   = "#include" , file_path ;
pp_define    = "#define" , identifier , [ "(" , id_list , ")" ] , replacement ;
pp_ifdef     = "#ifdef" , identifier ;
pp_ifndef    = "#ifndef" , identifier ;
pp_else      = "#else" ;
pp_endif     = "#endif" ;

file_path    = '"' , { ?any character except '"'? } , '"' ;
replacement  = ?rest of the line (line comments stripped, trimmed)? ;
(* every directive must begin its line: "#" starts a directive only when
   nothing but blanks precede it on that line *)

(* ===== Program =====

   Declarations and control constructs are themselves expressions:
   everything below that is reachable from `expr` may appear wherever
   an expression is expected, including inside blocks. *)

program      = { top_item } ;
top_item     = import_stmt | using_stmt | expr ;

(* ===== Expressions ===== *)

expr         = var_decl | const_def | fun_def | struct_def | union_def
             | enum_def | extern_var | typedef_def
             | if_expr | while_expr | for_expr | match_expr
             | return_expr | "break" | "continue"
             | assign_expr ;

assign_expr  = logical , [ assign_op , expr ] ;
(* target of "=" / op= must be a variable, index, member access or deref;
   assignment is right-associative *)

(* binary operators, lowest to highest precedence *)
logical      = logical_and , { ( "|" | "||" ) , logical_and } ;
logical_and  = comparison , { ( "&" | "&&" ) , comparison } ;
comparison   = bitwise , { ( "==" | "!=" | "<" | "<=" | ">" | ">=" ) , bitwise } ;
bitwise      = shift , { "^" , shift } ;
shift        = additive , { ( "<<" | ">>" ) , additive } ;
additive     = sum | range ;
range        = term , ".." , additive ;           (* left side stops at term level;
                                                     right-associative *)
sum          = term , { ( "+" | "-" ) , term } ;
term         = unary , { ( "*" | "/" | "%" ) , unary } ;
unary        = ( "$" | "*" | "&" | "-" | "+"
             | "!" | "~" | "++" | "--" ) , unary
             | postfix ;
(* $expr = deep copy; * = deref; & = address-of;
   "-" "+" "!" "~" cannot directly take "$" "*" "&" operands:
   "-*p" is rejected by the parser ("-(*p)" is fine),
   while "$" "*" "&" "++" "--" nest freely among themselves *)

postfix      = primary , { post_op } ;
post_op      = "(" , [ args ] , ")"              (* call *)
             | "[" , expr , "]"                  (* index *)
             | "." , identifier                  (* member access *)
             | "@" , type                        (* cast *)
             | "++" | "--" ;                     (* post-increment/decrement *)
args         = expr , { "," , expr } ;
(* trailing commas are tolerated in call/array/param/field/enum lists,
   but not inside generic "<...>" or the "using M::{...}" list *)
assign_op    = "=" | "+=" | "-=" | "*=" | "/=" | "%="
             | "&=" | "|=" | "^=" | "<<=" | ">>=" ;

primary      = int_lit | float_lit | bool_lit | nil_lit | string_lit | fstring_lit
             | "(" , logical , ")"
             | block
             | lambda
             | array_literal | array_fill
             | aggregate_literal
             | operand ;
(* parentheses wrap operator expressions only:
   control constructs, declarations and assignments cannot be parenthesized *)

block        = "{" , [ expr , { [ ";" ] , expr } ] , "}" ;

lambda       = "\" , "(" , params , ")" , ":" , type , expr ;

array_literal = "[" , [ expr , { "," , expr } ] , "]" ;
array_fill   = "[" , type , ";" , expr , "]" ;
(* the fill form is recognized only when the element type begins with a
   primitive type keyword, or an identifier immediately followed by ";
   e.g. "[P<int>; n]" does not parse as a fill *)

aggregate_literal
             = [ module_path , "::" ] , identifier , [ generic_args ] ,
               "{" , [ field_init , { "," , field_init } ] , "}" ;
(* only recognized when the name is a declared struct/union;
   generic_args are likewise consumed only in that context *)
field_init   = identifier , ":" , expr ;
operand      = [ module_path , "::" ] , identifier ;
module_path  = identifier , { "::" , identifier } ;
generic_args = "<" , type , { "," , type } , ">" ;
generic_params = "<" , id_list , ">" ;

if_expr      = "if" , expr , expr , [ "else" , expr ] ;
while_expr   = "while" , expr , expr ;
for_expr     = "for" , identifier , "in" , expr , expr ;
match_expr   = "match" , expr , "{" , { match_arm } , [ default_arm ] , "}" ;
match_arm    = expr , ":" , expr ;
default_arm  = "_" , ":" , expr ;                (* must be the last arm *)
return_expr  = "return" , [ expr ] ;

(* ===== Declarations ===== *)

fun_def      = "fun" , [ "(" , fn_anns , ")" ] , identifier ,
               [ generic_params ] ,
               "(" , params , ")" ,
               [ ":" , type ] ,
               [ expr ] ;                        (* body omitted iff annotated extern *)
fn_anns      = fn_ann , { "," , fn_ann } ;
fn_ann       = "pub" | "extern" | "pure" ;
params       = [ param , { "," , param } ] ;
param        = identifier , ":" , type           (* named *)
             | type ;                            (* anonymous *)

var_decl     = global_var | local_var ;
global_var   = [ "(" , "pub" , ")" ] , "var" , identifier , [ ":" , type ] , [ "=" , expr ] ;
local_var    = "var" , identifier , [ ":" , type ] , "=" , expr ;
const_def    = [ "(" , "pub" , ")" ] , "cst" , identifier , [ ":" , type ] , "=" , expr ;
(* "( pub )" is only valid at top level: writing "var ( pub )" or
   "cst ( pub )" inside a block is a syntax error *)
extern_var   = "extern" , identifier , ":" , type ;
typedef_def  = "typedef" , identifier , "=" , type ;

struct_def   = [ "(" , "pub" , ")" ] , "struct" , identifier ,
               [ generic_params ] , "{" , fields , "}" ;
union_def    = [ "(" , "pub" , ")" ] , "union" , identifier ,
               [ generic_params ] , "{" , fields , "}" ;
fields       = [ field , { "," , field } ] ;
field        = identifier , ":" , type ;

enum_def     = [ "(" , "pub" , ")" ] , "enum" , identifier ,
               "{" , [ enum_member , { "," , enum_member } ] , "}" ;
enum_member  = identifier , [ "=" , [ "-" ] , int_lit ] ;

id_list      = identifier , { "," , identifier } ;

(* ===== Types ===== *)

type         = "*" , type                        (* pointer *)
             | concrete_type ;
concrete_type = type_name , [ type_suffix ] ;
type_suffix   = "(" , type_params , ")"          (* function type *)
              | "[" , [ int_lit ] , "]" ;       (* array type, size ignored *)
type_params  = [ type , { "," , type } ] ;
type_name    = [ identifier , "::" ] , ( prim_type | identifier ) ,
               [ generic_args ] ;
prim_type    = "int" | "float" | "bool" | "string" | "void" ;
(* identifiers resolve to type parameters, typedefs, enums (as int),
   unions or structs, in that order;
   function-type and array suffixes are mutually exclusive *)

(* ===== Modules ===== *)

import_stmt  = "import" , module_path , [ "as" , identifier ] ;
(* path segments join into a module path "a/b/c";
   without "as", a multi-segment path is aliased to its last segment *)

using_stmt   = "using" , module_path , "::" , using_tail ;
using_tail   = "{" , using_item , { "," , using_item } , "}"
             | identifier , [ "as" , identifier ] ;
using_item   = identifier , [ "as" , identifier ] ;
(* "using M::name" imports one member; "using M::{a, b as c}" imports several;
   if the name after "::" resolves to a sub-module file,
   the whole sub-module is imported instead;
   the braced list requires a single-segment module name *)
```
