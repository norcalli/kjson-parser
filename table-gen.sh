#!/usr/bin/env fish
printf "pub const HEXDIGIT_TABLE: [bool; 256] = %s;\n" (jq -nc '[range(256) | (. >= 48 and . <= 57) or (. >= 65 and . <= 70) or (. >= 97 and . <= 102)]') > src/lookup_tables.rs
printf "pub const DIGIT_TABLE: [bool; 256] = %s;\n" (jq -nc '[range(256) | (. >= 48 and . <= 57)]') >> src/lookup_tables.rs
printf "pub const NONZERO_DIGIT_TABLE: [bool; 256] = %s;\n" (jq -nc '[range(256) | (. > 48 and . <= 57)]') >> src/lookup_tables.rs
# 92 == \
# 34 == "
printf "pub const STRING_TERMINALS: [bool; 256] = %s;\n" (jq -nc '[range(256) | (. > 127 or . == 92 or . == 34)]') >> src/lookup_tables.rs
