use super::error::PreprocessorError;
use crate::compiler::{
    SourceMap,
    preprocessor::{MacroDefinition, Preprocessor},
};
use std::{fs, path::Path};

impl<'a> Preprocessor<'a> {
    fn emit_char(&self, ch: char, output: &mut String, map: &mut SourceMap) {
        if ch == '\n' {
            map.record_line(&self.base_path, self.row);
        }
        output.push(ch);
    }

    fn emit_str(&self, s: &str, output: &mut String, map: &mut SourceMap) {
        for ch in s.chars() {
            self.emit_char(ch, output, map);
        }
    }

    fn current(&mut self) -> char {
        *self.src.peek().unwrap_or(&'\0')
    }

    fn bump(&mut self) {
        if let Some(c) = self.src.next() {
            if c == '\n' {
                self.row += 1;
                self.col = 0;
            } else {
                self.col += 1;
            }
        }
    }

    fn skip_spaces(&mut self) {
        while self.current() == ' ' || self.current() == '\t' {
            self.bump();
        }
    }

    fn skip_until_newline(&mut self) {
        while self.current() != '\n' && self.current() != '\0' {
            self.bump();
        }
    }

    fn parse_ident(&mut self) -> String {
        let mut ident = String::new();
        if self.current().is_ascii_alphabetic() || self.current() == '_' {
            ident.push(self.current());
            self.bump();
        }
        while self.current().is_alphanumeric() || self.current() == '_' {
            ident.push(self.current());
            self.bump();
        }
        ident
    }

    fn parse_file_path(&mut self) -> Option<String> {
        self.skip_spaces();
        if self.current() != '"' {
            return None;
        }
        self.bump();
        let mut file = String::new();
        while self.current() != '"' && self.current() != '\0' {
            file.push(self.current());
            self.bump();
        }
        if self.current() == '"' {
            self.bump();
            return Some(file);
        }
        None
    }

    fn expand_macros(&self, value: &str) -> String {
        let mut result = value.to_string();
        let mut changed = true;
        let max_iterations = 100;
        let mut iterations = 0;

        while changed && iterations < max_iterations {
            changed = false;

            for (name, macro_def) in &self.defines {
                let macro_pattern = format!("{}(", name);
                let mut search_start = 0;

                while let Some(abs_pos) =
                    self.find_outside_strings(&result, &macro_pattern, search_start)
                {
                    let before = result[..abs_pos].chars().next_back();
                    if before.map_or(false, |c| c.is_alphanumeric() || c == '_') {
                        search_start = abs_pos + 1;
                        continue;
                    }
                    let args_start = abs_pos + name.len();

                    let (args_str, args_end) = match self.extract_args(&result, args_start) {
                        Some(args) => args,
                        None => {
                            search_start = abs_pos + 1;
                            continue;
                        }
                    };

                    let args = self.split_macro_args(&args_str);

                    if args.len() != macro_def.params.len() {
                        search_start = abs_pos + 1;
                        continue;
                    }

                    let expanded_body =
                        self.substitute_params(&macro_def.body, &macro_def.params, &args);

                    if expanded_body.starts_with(&format!("{}({})", name, args_str)) {
                        search_start = abs_pos + name.len();
                        continue;
                    }

                    let before = &result[..abs_pos];
                    let after = &result[args_end..];
                    result = format!("{}{}{}", before, expanded_body, after);

                    changed = true;
                    search_start = abs_pos + expanded_body.len();
                }
            }

            for (name, macro_def) in &self.defines {
                if macro_def.params.is_empty() {
                    let mut new_result = String::new();
                    let mut i = 0;
                    let mut in_string = false;
                    let mut quote = '\0';
                    while i < result.len() {
                        let ch = result[i..].chars().next().unwrap();
                        if in_string {
                            new_result.push(ch);
                            i += ch.len_utf8();
                            if ch == '\\' {
                                if let Some(esc) = result[i..].chars().next() {
                                    new_result.push(esc);
                                    i += esc.len_utf8();
                                }
                            } else if ch == quote {
                                in_string = false;
                            }
                            continue;
                        }
                        if result[i..].starts_with(name.as_str()) {
                            let end = i + name.len();
                            let before = result[..i].chars().next_back();
                            let after = result[end..].chars().next();
                            let is_ident = |c: Option<char>| {
                                c.map_or(false, |c| c.is_alphanumeric() || c == '_')
                            };

                            if !is_ident(before) && !is_ident(after) {
                                new_result.push_str(&macro_def.body);
                                i = end;
                                changed = true;
                                continue;
                            }
                        }
                        if ch == '"' || ch == '\'' {
                            in_string = true;
                            quote = ch;
                        }
                        new_result.push(ch);
                        i += ch.len_utf8();
                    }
                    result = new_result;
                }
            }

            iterations += 1;
        }

        if changed {
            eprintln!(
                "warning: macro expansion did not converge after {max_iterations} rounds (recursive macro?)"
            );
        }

        result
    }

    fn find_outside_strings(&self, text: &str, pattern: &str, start: usize) -> Option<usize> {
        let mut i = start;
        let mut in_string = false;
        let mut quote = '\0';
        while i < text.len() {
            let ch = text[i..].chars().next()?;
            if in_string {
                if ch == '\\' {
                    i += ch.len_utf8();
                    if let Some(esc) = text[i..].chars().next() {
                        i += esc.len_utf8();
                    }
                    continue;
                }
                if ch == quote {
                    in_string = false;
                }
                i += ch.len_utf8();
                continue;
            }
            if text[i..].starts_with(pattern) {
                return Some(i);
            }
            if ch == '"' || ch == '\'' {
                in_string = true;
                quote = ch;
            }
            i += ch.len_utf8();
        }
        None
    }

    fn split_macro_args(&self, args_str: &str) -> Vec<String> {
        if args_str.trim().is_empty() {
            return Vec::new();
        }
        let mut args = Vec::new();
        let mut depth: isize = 0;
        let mut in_string = false;
        let mut quote = '\0';
        let mut current = String::new();
        let mut chars = args_str.chars().peekable();
        while let Some(c) = chars.next() {
            if in_string {
                current.push(c);
                if c == '\\' {
                    if let Some(&next) = chars.peek() {
                        current.push(next);
                        chars.next();
                    }
                } else if c == quote {
                    in_string = false;
                }
                continue;
            }
            match c {
                '"' | '\'' => {
                    in_string = true;
                    quote = c;
                    current.push(c);
                }
                '(' | '[' | '{' => {
                    depth += 1;
                    current.push(c);
                }
                ')' | ']' | '}' => {
                    depth -= 1;
                    current.push(c);
                }
                ',' if depth == 0 => {
                    args.push(current.trim().to_string());
                    current = String::new();
                }
                _ => current.push(c),
            }
        }
        args.push(current.trim().to_string());
        args
    }

    fn extract_args(&self, text: &str, start: usize) -> Option<(String, usize)> {
        if start >= text.len() || text.as_bytes()[start] != b'(' {
            return None;
        }

        let mut depth = 1;
        let args_start = start + 1;
        let mut args_end = start;
        let mut found = false;
        let mut in_string = false;
        let mut string_char = '\0';
        let mut chars = text[start + 1..].char_indices().peekable();

        while let Some((i, c)) = chars.next() {
            let pos = start + 1 + i;

            if c == '\\' && in_string {
                chars.next();
                continue;
            }

            if c == '"' || c == '\'' {
                if !in_string {
                    in_string = true;
                    string_char = c;
                } else if c == string_char {
                    in_string = false;
                }
            }

            if !in_string {
                if c == '(' {
                    depth += 1;
                } else if c == ')' {
                    if depth == 1 {
                        args_end = pos;
                        found = true;
                        break;
                    }
                    depth -= 1;
                }
            }
        }

        if !found {
            return None;
        }

        let args_str = text[args_start..args_end].trim().to_string();
        Some((args_str, args_end + 1))
    }

    fn substitute_params(&self, body: &str, params: &[String], args: &[String]) -> String {
        let is_ident = |c: Option<char>| c.map_or(false, |c| c.is_alphanumeric() || c == '_');
        let mut result = String::new();
        let mut i = 0;
        while i < body.len() {
            let mut matched = false;
            for (param, arg) in params.iter().zip(args.iter()) {
                if body[i..].starts_with(param.as_str()) {
                    let end = i + param.len();
                    let before = body[..i].chars().next_back();
                    let after = body[end..].chars().next();
                    if !is_ident(before) && !is_ident(after) {
                        result.push_str(arg);
                        i = end;
                        matched = true;
                        break;
                    }
                }
            }
            if !matched {
                let ch = body[i..].chars().next().unwrap();
                result.push(ch);
                i += ch.len_utf8();
            }
        }

        result
    }

    fn check_condition(&mut self, negated: bool) -> bool {
        self.skip_spaces();
        let ident = self.parse_ident();
        let defined = self.defines.contains_key(&ident);

        if negated { !defined } else { defined }
    }

    fn find_import_file(&self, file_name: &str) -> Option<String> {
        let input_dir = if self.base_path.is_empty() {
            ".".to_string()
        } else {
            Path::new(&self.base_path)
                .parent()
                .and_then(|p| p.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(".")
                .to_string()
        };

        let mut search_paths = vec![
            format!("{}/{}", input_dir, file_name),
            format!("{}/{}.al", input_dir, file_name),
            format!("{}/{}.ah", input_dir, file_name),
        ];

        for path in &self.include_paths {
            search_paths.push(format!("{}/{}", path, file_name));
            search_paths.push(format!("{}/{}.al", path, file_name));
            search_paths.push(format!("{}/{}.ah", path, file_name));
        }

        for path in &search_paths {
            if Path::new(path).exists() {
                return Some(path.clone());
            }
        }

        None
    }

    fn import_file_key(path: &str) -> Option<String> {
        fs::canonicalize(path)
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
    }

    fn strip_line_comment(s: &str) -> String {
        let mut in_string = false;
        let mut quote = '\0';
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < s.len() {
            let c = s[i..].chars().next().unwrap();
            if in_string {
                if c == '\\' {
                    i += 1;
                } else if c == quote {
                    in_string = false;
                }
                i += c.len_utf8();
                continue;
            }
            if c == '"' || c == '\'' {
                in_string = true;
                quote = c;
                i += c.len_utf8();
                continue;
            }
            if c == '/' && i + 1 < s.len() && bytes[i + 1] == b'/' {
                return s[..i].to_string();
            }
            i += c.len_utf8();
        }
        s.to_string()
    }

    pub fn preprocess(&mut self) -> Result<(String, SourceMap), PreprocessorError> {
        let mut output = String::new();
        let mut source_map = SourceMap::new();
        source_map.add_file(self.base_path.clone(), self.source_text.to_string());

        if self.import_chain.is_empty()
            && let Some(key) = Self::import_file_key(&self.base_path)
        {
            self.import_chain.push(key);
        }

        while self.current() != '\0' {
            if self.current() == '/' {
                self.bump();
                if self.current() == '/' {
                    while self.current() != '\n' && self.current() != '\0' {
                        self.bump();
                    }
                    continue;
                }
                self.emit_char('/', &mut output, &mut source_map);
                continue;
            }

            if self.current() == '#' {
                let out_tail = output.as_bytes();
                let line_start = match out_tail.iter().rposition(|&b| b == b'\n') {
                    Some(i) => i + 1,
                    None => 0,
                };
                if !out_tail[line_start..]
                    .iter()
                    .all(|&b| b == b' ' || b == b'\t')
                {
                    self.emit_char('#', &mut output, &mut source_map);
                    continue;
                }

                self.bump();
                let cmd = self.parse_ident();

                match cmd.as_str() {
                    "define" => {
                        if self.skipping {
                            self.skip_until_newline();
                            continue;
                        }
                        self.skip_spaces();
                        let name = self.parse_ident();
                        self.skip_spaces();

                        let (params, value) = if self.current() == '(' {
                            self.bump();
                            let mut params = Vec::new();
                            let mut current_param = String::new();

                            while self.current() != ')' && self.current() != '\0' {
                                if self.current() == ',' {
                                    if !current_param.trim().is_empty() {
                                        params.push(current_param.trim().to_string());
                                    }
                                    current_param.clear();
                                    self.bump();
                                    self.skip_spaces();
                                } else if self.current().is_alphanumeric() || self.current() == '_'
                                {
                                    current_param.push(self.current());
                                    self.bump();
                                } else if !self.current().is_whitespace() {
                                    return Err(PreprocessorError::MacroError {
                                        msg: format!(
                                            "Invalid parameter name: '{}'",
                                            self.current()
                                        ),
                                        row: self.row,
                                        col: self.col,
                                    });
                                } else {
                                    self.bump();
                                }
                            }

                            if !current_param.trim().is_empty() {
                                params.push(current_param.trim().to_string());
                            }

                            if self.current() != ')' {
                                return Err(PreprocessorError::MacroError {
                                    msg: "Unclosed parameter list in macro definition".to_string(),
                                    row: self.row,
                                    col: self.col,
                                });
                            }
                            self.bump();
                            self.skip_spaces();

                            let mut macro_body = String::new();
                            while self.current() != '\n' && self.current() != '\0' {
                                macro_body.push(self.current());
                                self.bump();
                            }

                            (
                                params,
                                Self::strip_line_comment(&macro_body).trim().to_string(),
                            )
                        } else {
                            let mut simple_value = String::new();
                            while self.current() != '\n' && self.current() != '\0' {
                                simple_value.push(self.current());
                                self.bump();
                            }
                            (
                                Vec::new(),
                                Self::strip_line_comment(&simple_value).trim().to_string(),
                            )
                        };

                        let expanded_value = self.expand_macros(&value);

                        self.defines.insert(
                            name,
                            MacroDefinition {
                                params,
                                body: expanded_value,
                            },
                        );
                    }
                    "ifdef" => {
                        if self.skipping {
                            self.condition_stack.push(false);
                            self.skip_until_newline();
                            continue;
                        }
                        let condition_met = self.check_condition(false);
                        self.condition_stack.push(condition_met);
                        self.skipping = !condition_met;
                        self.skip_until_newline();
                    }
                    "ifndef" => {
                        if self.skipping {
                            self.condition_stack.push(false);
                            self.skip_until_newline();
                            continue;
                        }
                        let condition_met = self.check_condition(true);
                        self.condition_stack.push(condition_met);
                        self.skipping = !condition_met;
                        self.skip_until_newline();
                    }
                    "else" => {
                        if let Some(top) = self.condition_stack.last_mut() {
                            *top = !*top;
                        } else {
                            return Err(PreprocessorError::ConditionError {
                                msg: "Unexpected $else".to_string(),
                                row: self.row,
                                col: self.col,
                            });
                        }
                        self.skip_until_newline();
                        self.skipping = self.condition_stack.iter().any(|&active| !active);
                    }
                    "endif" => {
                        if let Some(_) = self.condition_stack.pop() {
                            self.skipping = self.condition_stack.iter().any(|&active| !active);
                        } else {
                            return Err(PreprocessorError::ConditionError {
                                msg: "Unexpected $endif".to_string(),
                                row: self.row,
                                col: self.col,
                            });
                        }
                    }
                    "include" => {
                        if self.skipping {
                            self.skip_until_newline();
                            continue;
                        }

                        let file_name =
                            self.parse_file_path().ok_or(PreprocessorError::IoError {
                                msg: "Invalid import path".to_string(),
                                row: self.row,
                                col: self.col,
                            })?;

                        let file_path = self.find_import_file(&file_name).ok_or(
                            PreprocessorError::ImportError {
                                file: file_name.clone(),
                                row: self.row,
                                col: self.col,
                            },
                        )?;

                        let import_key =
                            Self::import_file_key(&file_path).unwrap_or_else(|| file_path.clone());
                        if self.import_chain.iter().any(|p| *p == import_key) {
                            return Err(PreprocessorError::ImportError {
                                file: file_name.clone(),
                                row: self.row,
                                col: self.col,
                            });
                        }

                        let content = fs::read_to_string(&file_path).map_err(|e| {
                            PreprocessorError::IoError {
                                msg: format!("Failed to read file: {}", e),
                                row: self.row,
                                col: self.col,
                            }
                        })?;

                        if self.import_chain.len() >= 32 {
                            return Err(PreprocessorError::IoError {
                                msg: format!(
                                    "import depth exceeds 32 levels (circular or runaway includes?)"
                                ),
                                row: self.row,
                                col: self.col,
                            });
                        }
                        let mut child_pp = Preprocessor::new(
                            &content,
                            file_path.clone(),
                            self.include_paths.clone(),
                        );
                        child_pp.defines = self.defines.clone();
                        let mut child_chain = self.import_chain.clone();
                        child_chain.push(import_key);
                        child_pp.import_chain = child_chain;
                        let (processed_sub, child_map) = child_pp.preprocess()?;
                        source_map.merge_child(child_map);
                        output.push_str(&processed_sub);
                        self.defines = child_pp.defines;
                    }
                    _ => {
                        if self.skipping {
                            self.skip_until_newline();
                            continue;
                        }

                        let macro_body = self
                            .defines
                            .get(&cmd)
                            .filter(|m| m.params.is_empty())
                            .map(|m| m.body.clone());
                        if let Some(body) = macro_body {
                            self.emit_str(&body, &mut output, &mut source_map);
                        } else {
                            self.emit_char('$', &mut output, &mut source_map);
                            self.emit_str(&cmd, &mut output, &mut source_map);
                        }
                    }
                }
            } else {
                if self.skipping {
                    self.bump();
                    continue;
                }

                if self.current() == '"' || self.current() == '\'' {
                    let quote = self.current();
                    self.emit_char(quote, &mut output, &mut source_map);
                    self.bump();
                    while self.current() != '\0' {
                        let c = self.current();
                        if c == '\\' {
                            self.emit_char(c, &mut output, &mut source_map);
                            self.bump();
                            if self.current() != '\0' {
                                let esc = self.current();
                                self.emit_char(esc, &mut output, &mut source_map);
                                self.bump();
                            }
                            continue;
                        }
                        self.emit_char(c, &mut output, &mut source_map);
                        self.bump();
                        if c == quote {
                            break;
                        }
                    }
                    continue;
                }

                if self.current().is_ascii_alphabetic() || self.current() == '_' {
                    let ident = self.parse_ident();
                    self.emit_str(&ident, &mut output, &mut source_map);
                } else {
                    let ch = self.current();
                    self.emit_char(ch, &mut output, &mut source_map);
                    self.bump();
                }
            }
        }

        if !self.condition_stack.is_empty() {
            return Err(PreprocessorError::ConditionError {
                msg: "Unclosed $ifdef or $ifndef".to_string(),
                row: self.row,
                col: self.col,
            });
        }

        let expanded_output = self.expand_macros(&output);
        Ok((expanded_output, source_map))
    }
}
