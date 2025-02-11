import gleam/dict
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/string_tree.{
  type StringTree, append, append_tree, from_string, from_strings,
}
import graph.{type Graph, type Node}
import syntax

pub fn to_json(graph: Graph) -> String {
  let nodes =
    list.map(graph.nodes, node_to_json)
    |> json_array

  let outputs =
    dict.to_list(graph.outputs)
    |> pairs_to_sparse_list
    |> list.map(json_map_option(_, json_int))
    |> json_array

  json_object([#("nodes", nodes), #("outputs", outputs)])
  |> string_tree.to_string
}

fn node_to_json(node: Node) -> StringTree {
  let #(typ, fields) = case node {
    graph.Const(value) -> #("const", [#("value", json_int(value))])
    graph.Input(name) -> #("input", [#("name", json_string(name))])
    graph.Math(left, op, right) -> #("math", [
      #("left", json_int(left)),
      #("operator", json_string(math_operator_name(op))),
      #("right", json_int(right)),
    ])
    graph.Compare(left, op, right) -> #("compare", [
      #("left", json_int(left)),
      #("operator", json_string(comparison_name(op))),
      #("right", json_int(right)),
    ])
    graph.Boolean(left, op, right) -> #("boolean", [
      #("left", json_int(left)),
      #("operator", json_string(boolean_operator_name(op))),
      #("right", json_int(right)),
    ])
    graph.BooleanNot(of) -> #("boolean", [
      #("operator", json_string("not")),
      #("of", json_int(of)),
    ])
    graph.Switch(condition, high, low) -> #("switch", [
      #("condition", json_int(condition)),
      #("high", json_int(high)),
      #("low", json_int(low)),
    ])
  }

  let fields = [#("type", json_string(typ)), ..fields]

  json_object(fields)
}

fn pairs_to_sparse_list(pairs: List(#(Int, a))) -> List(Option(a)) {
  pairs
  |> list.sort(fn(a, b) { int.compare(a.0, b.0) })
  |> list.fold([], fn(acc, pair) {
    [Some(pair.1), ..extend(acc, None, pair.0 - 1 - list.length(acc))]
  })
  |> list.reverse
}

fn extend(list: List(a), with: a, count: Int) -> List(a) {
  case count {
    x if x <= 0 -> list
    _ -> [with, ..list] |> extend(with, count - 1)
  }
}

fn math_operator_name(op: syntax.MathOperator) -> String {
  case op {
    syntax.Add -> "add"
    syntax.Subtract -> "sub"
  }
}

fn comparison_name(op: syntax.Comparison) -> String {
  case op {
    syntax.LessThan -> "<"
    syntax.LessThanOrEqual -> "<="
    syntax.GreaterThan -> ">"
    syntax.GreaterThanOrEqual -> ">="
    syntax.EqualTo -> "=="
    syntax.NotEqualTo -> "!="
  }
}

fn boolean_operator_name(op: syntax.BooleanOperator) -> String {
  case op {
    syntax.And -> "and"
    syntax.Or -> "or"
  }
}

fn json_string(s: String) -> StringTree {
  from_strings(["\"", s, "\""])
}

fn json_int(i: Int) -> StringTree {
  from_string(int.to_string(i))
}

fn json_map_option(x: Option(a), map: fn(a) -> StringTree) -> StringTree {
  option.map(x, map)
  |> option.lazy_unwrap(fn() { string_tree.from_string("null") })
}

fn json_object(fields: List(#(String, StringTree))) -> StringTree {
  let strs =
    list.map(fields, fn(field) {
      json_string(field.0) |> append(":") |> append_tree(field.1)
    })
    |> list.intersperse(from_string(","))

  from_string("{")
  |> list.fold(strs, _, append_tree)
  |> append("}")
}

fn json_array(items: List(StringTree)) -> StringTree {
  let items = list.intersperse(items, from_string(","))

  from_string("[")
  |> list.fold(items, _, append_tree)
  |> append("]")
}
