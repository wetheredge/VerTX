import gleam/option.{None, Some}
import gleam/set
import nibble
import nibble/lexer

pub type Expr =
  List(Node)

pub type Node {
  Input(String)
  Output(Int)
  Get(String)
  Set(String)
  Const(Int)
  Math(operator: MathOperator, rhs: Expr)
  Switch(high: Expr, low: Expr)
}

pub type MathOperator {
  Add
  Subtract
}

pub type Token {
  TNum(Int)
  Ident(String)
  LParen
  Comma
  RParen
  Arrow
  Semicolon
  Comment(String)
}

pub fn node_name(node: Node) -> String {
  case node {
    Const(..) -> "const"
    Get(..) -> "get"
    Input(..) -> "input"
    Output(..) -> "output"
    Set(..) -> "set"
    Math(..) -> "math"
    Switch(..) -> "switch"
  }
}

pub fn lex(input: String) -> Result(List(lexer.Token(Token)), lexer.Error) {
  let lexer =
    lexer.simple([
      lexer.int(TNum),
      lexer.variable(set.new(), Ident),
      lexer.token("(", LParen),
      lexer.token(",", Comma),
      lexer.token(")", RParen),
      lexer.token("->", Arrow),
      lexer.token(";", Semicolon),
      lexer.whitespace(Nil) |> lexer.ignore,
      lexer.comment("#", Comment) |> lexer.ignore,
    ])

  lexer.run(input, lexer)
}

pub fn node_parser() -> nibble.Parser(Node, Token, Nil) {
  let node_name_parser = {
    use token <- nibble.take_map("expected node name")
    case token {
      Ident(node) -> Some(node)
      _ -> None
    }
  }

  let comma_parser = {
    use token <- nibble.take_map("expected comma")
    case token {
      Comma -> Some(Nil)
      _ -> None
    }
  }

  use node <- nibble.do(node_name_parser)
  use _ <- nibble.do(nibble.token(LParen))

  use node <- nibble.do(case node {
    "input" -> take_ident("input name", Input)
    "output" -> {
      use token <- nibble.take_map("output channel number (1..)")
      case token {
        TNum(n) if n > 0 -> Some(Output(n))
        _ -> None
      }
    }
    "get" -> take_ident("variable name", Get)
    "set" -> take_ident("variable name", Set)
    "const" -> {
      use token <- nibble.take_map("integer")
      case token {
        TNum(n) -> Some(Const(n))
        _ -> None
      }
    }
    "math" -> {
      use op <- nibble.do({
        use token <- nibble.take_map("operator")
        case token {
          Ident("add") -> Some(Add)
          Ident("subtract") | Ident("sub") -> Some(Subtract)
          _ -> None
        }
      })
      use _ <- nibble.do(comma_parser)
      use rhs <- nibble.do(expression_parser())

      nibble.succeed(Math(op, rhs))
    }
    "switch" -> {
      use high <- nibble.do(expression_parser())
      use _ <- nibble.do(comma_parser)
      use low <- nibble.do(expression_parser())
      use _ <- nibble.do(nibble.optional(comma_parser))
      nibble.succeed(Switch(high, low))
    }
    _ -> nibble.fail("unknown node name: " <> node)
  })

  use _ <- nibble.do(nibble.token(RParen))
  nibble.return(node)
}

fn take_ident(
  expected: String,
  map: fn(String) -> a,
) -> nibble.Parser(a, Token, Nil) {
  use token <- nibble.take_map("expected " <> expected)
  case token {
    Ident(ident) -> Some(map(ident))
    _ -> None
  }
}

pub fn expression_parser() -> nibble.Parser(Expr, Token, Nil) {
  let arrow =
    nibble.take_map("expected `->`", fn(token) {
      case token {
        Arrow -> Some(Nil)
        _ -> None
      }
    })

  nibble.sequence(node_parser(), arrow)
}

pub fn parse(
  tokens: List(lexer.Token(Token)),
) -> Result(List(Expr), List(nibble.DeadEnd(Token, Nil))) {
  let semicolon =
    nibble.take_map("expected semicolon", fn(token) {
      case token {
        Semicolon -> Some(Nil)
        _ -> None
      }
    })

  let expr_semi = {
    use expr <- nibble.do(expression_parser())
    use _ <- nibble.do(semicolon)
    nibble.return(expr)
  }

  let parser = {
    use exprs <- nibble.do(nibble.many(expr_semi))
    use _ <- nibble.do(nibble.eof())
    nibble.return(exprs)
  }

  nibble.run(tokens, parser)
}
