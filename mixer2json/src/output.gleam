import gleam/dict
import gleam/int
import gleam/list
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
    |> list.sort(fn(a, b) { int.compare(a.0, b.0) })
    |> list.map(fn(pair) { #(int.to_string(pair.0), json_int(pair.1)) })
    |> json_object

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
    graph.Switch(condition, high, low) -> #("switch", [
      #("condition", json_int(condition)),
      #("high", json_int(high)),
      #("low", json_int(low)),
    ])
  }

  let fields = [#("type", json_string(typ)), ..fields]

  json_object(fields)
}

fn math_operator_name(op: syntax.MathOperator) -> String {
  case op {
    syntax.Add -> "add"
    syntax.Subtract -> "sub"
  }
}

fn json_string(s: String) -> StringTree {
  from_strings(["\"", s, "\""])
}

fn json_int(i: Int) -> StringTree {
  from_string(int.to_string(i))
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
