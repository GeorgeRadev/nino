import moo from "moo";

let lexer = moo.compile({
    ID: /[a-zA-Z_][a-zA-Z0-9_]*/,
    N: /0|[1-9][0-9]*/,
    S1: /"(?:\\["\\]|[^\n"\\])*"/,
    S2: /'(?:\\['\\]|[^\n'\\])*'/,
    S3: { match: /`(?:\\[`\\]|[^\n`\\])*`/, lineBreaks: true }, // multiline string
    LP: '(',
    RP: ')',
    C: ':',
    SC: ';',
    COMMENT1: /\/\/.*?$/,
    WS: /[ \t]+/, // white space
    NL: { match: /\r?\n/, lineBreaks: true }, // new line
    COMMENT2: { match: /\/\*.*\*\//, lineBreaks: true },
    FILL: { match: /.{1}/, lineBreaks: true },
})

function isSQLStart(token) {
    const token_len = token.length;
    if (token_len < 3) {
        return false;
    }
    const c0 = token.charAt(0);
    switch (c0) {
        case 'A': return "ALTER" == token;
        case 'C': {
            const c1 = token.charAt(1);
            if (c1 == 'O') return "COMMIT" == token;
            if (c1 == 'R') return "CREATE" == token;
            return false;
        }
        case 'D': return "DELETE" == token;
        case 'E': return "EXECUTE" == token;
        case 'G': return "GRANT" == token;
        case 'I': return "INSERT" == token;
        case 'R': return "ROLLBACK" == token;
        case 'S': {
            const c2 = token.charAt(2);
            if (c2 == 'L') return "SELECT" == token;
            if (c2 == 'T') return "SET" == token;
            return false;
        }
        case 'U': return "UPDATE" == token;
    }
    return false;
}

export function sqlToArray(code) {
    var result = "";
    lexer.reset(code)
    while (true) {
        var t = lexer.next();
        if (!t) break;
        if (t.type == "ID" && isSQLStart(t.value)) {
            const start = t.offset;
            var sql_string = t.value;
            var variables = [];
            // read until ; and collect all variables
            while (true) {
                t = lexer.next();
                if (!t) {
                    throw Error("No terminating semicolumn for SQL statement starting at " + start);

                } else if (t.type == "SC") {
                    //create the array presentation
                    // convert from:
                    // SELECT id, username FROM users WHERE active = :active;
                    // into:
                    // [ "SELECT id, username FROM users WHERE active = $1;", active ]
                    result += "[ \`";
                    result += sql_string;
                    result += "\`";
                    for (var v of variables) {
                        result += ", " + v;
                    }
                    result += "] ";

                    break;
                } else if (t.type == "C") {
                    // read ID or expression and move it as array parameter
                    const c_start = t.offset;
                    t = lexer.next();
                    if (t.type == "ID") {
                        variables.push(t.value);
                        sql_string += " $" + variables.length + " ";
                    } else if (t.type == "LP") {
                        var bracket_open_closed = 1;
                        var val = t.value;
                        const bracket_start = t.offset;
                        // read untill closing brackets are encountered
                        while (true) {
                            t = lexer.next();
                            if (!t) {
                                throw Error("No closing bracket for expression starting at " + bracket_start);
                            } else if (t.type == "LP") {
                                bracket_open_closed++;
                            } else if (t.type == "RP") {
                                bracket_open_closed--;
                                if (bracket_open_closed <= 0) {
                                    val += t.value;
                                    variables.push(val);
                                    sql_string += " $" + variables.length + " ";
                                    break;
                                }
                            } else {
                                val += t.value;
                            }
                        }
                    } else {
                        throw Error("Expecting identifyer or bracketed expression at " + c_start);
                    }

                } else if (t.type == "S1" || t.type == "S2") {
                    // string is going as parameter
                    variables.push(t.value);
                    sql_string += " $" + variables.length + " ";

                } else {
                    // just append to the string content
                    sql_string += t.value;
                }
            }
        } else {
            result += t.value;
        }
    }
    return result;
}