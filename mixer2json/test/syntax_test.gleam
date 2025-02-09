import gleeunit/should
import nibble
import syntax

fn lex_and_parse_node(
  input: String,
) -> Result(syntax.Node, List(nibble.DeadEnd(syntax.Token, Nil))) {
  let assert Ok(tokens) = syntax.lex(input)
  nibble.run(tokens, syntax.node_parser())
}

fn lex_and_parse_expr(
  input: String,
) -> Result(List(syntax.Node), List(nibble.DeadEnd(syntax.Token, Nil))) {
  let assert Ok(tokens) = syntax.lex(input)
  nibble.run(tokens, syntax.expression_parser())
}

pub fn input_test() {
  lex_and_parse_node("input(throttle)")
  |> should.equal(Ok(syntax.Input("throttle")))
}

pub fn variables_test() {
  lex_and_parse_node("set(test)")
  |> should.equal(Ok(syntax.Set("test")))

  lex_and_parse_node("get(test)")
  |> should.equal(Ok(syntax.Get("test")))
}

pub fn output_test() {
  lex_and_parse_node("output(0)")
  |> should.be_error()

  lex_and_parse_node("output(1)")
  |> should.equal(Ok(syntax.Output(1)))
}

pub fn const_test() {
  lex_and_parse_node("const(42)")
  |> should.equal(Ok(syntax.Const(42)))
}

pub fn math_test() {
  lex_and_parse_node("math(add, const(1))")
  |> should.equal(Ok(syntax.Math(syntax.Add, [syntax.Const(1)])))

  lex_and_parse_node("math(sub, const(1))")
  |> should.equal(Ok(syntax.Math(syntax.Subtract, [syntax.Const(1)])))

  lex_and_parse_node("math(subtract, const(1))")
  |> should.equal(Ok(syntax.Math(syntax.Subtract, [syntax.Const(1)])))
}

pub fn switch_test() {
  let expected = syntax.Switch([syntax.Const(10)], [syntax.Const(20)])

  lex_and_parse_node("switch(const(10), const(20))")
  |> should.equal(Ok(expected))

  lex_and_parse_node("switch(const(10), const(20),)")
  |> should.equal(Ok(expected))
}

pub fn expression_test() {
  lex_and_parse_expr("get(foo) -> set(bar)")
  |> should.equal(Ok([syntax.Get("foo"), syntax.Set("bar")]))
}
