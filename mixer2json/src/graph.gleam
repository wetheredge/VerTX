import gleam/dict.{type Dict}
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/set.{type Set}
import syntax

pub type Graph {
  Graph(nodes: List(Node), outputs: Dict(Int, NodeId))
}

pub type NodeId =
  Int

pub type GenericNode(link) {
  Input(name: String)
  Const(value: Int)
  Math(left: link, op: syntax.MathOperator, right: link)
  Switch(condition: link, high: link, low: link)
}

pub type Node =
  GenericNode(NodeId)

type LazyNode =
  GenericNode(LazyLink)

type LazyLink {
  KnownLink(NodeId)
  LazyLink(var: String)
}

type State {
  State(
    nodes: List(LazyNode),
    outputs: Dict(Int, LazyLink),
    vars: Vars,
    last_id: Option(LazyLink),
  )
}

type Vars =
  Dict(String, LazyLink)

fn add_node(s: State, node: LazyNode) -> State {
  State(
    ..s,
    nodes: [node, ..s.nodes],
    last_id: Some(KnownLink(list.length(s.nodes))),
  )
}

fn take_last_id(s: State) -> #(State, LazyLink) {
  let last_id = case s.last_id {
    Some(id) -> id
    None -> panic as "node expected input"
  }
  #(State(..s, last_id: None), last_id)
}

pub fn flatten(mixer: List(syntax.Expr)) -> Graph {
  let result = {
    use s, expr <- list.fold(mixer, State([], dict.new(), dict.new(), None))
    process_expr(s, expr)
  }

  let vars = result.vars
  let resolve_link = resolve_link(vars, _)

  let nodes =
    list.reverse(result.nodes)
    |> list.map(fn(node) {
      case node {
        Const(value) -> Const(value)
        Input(name) -> Input(name)
        Math(left, op, right) ->
          Math(resolve_link(left), op, resolve_link(right))
        Switch(condition, high, low) ->
          Switch(resolve_link(condition), resolve_link(high), resolve_link(low))
      }
    })

  let outputs =
    dict.map_values(result.outputs, fn(_, link) { resolve_link(link) })

  Graph(nodes, outputs)
}

fn process_expr(s: State, expr: syntax.Expr) -> State {
  use s, node <- list.fold(expr, s)
  case node {
    syntax.Const(value) -> add_node(s, Const(value))
    syntax.Get(var) -> State(..s, last_id: Some(LazyLink(var)))
    syntax.Input(name) -> add_node(s, Input(name))
    syntax.Output(channel) -> {
      let #(s, last_id) = take_last_id(s)
      State(..s, outputs: dict.insert(s.outputs, channel, last_id))
    }
    syntax.Set(var) -> {
      let #(s, last_id) = take_last_id(s)
      State(..s, vars: dict.insert(s.vars, var, last_id))
    }
    syntax.Math(op, rhs) -> {
      let #(s, left) = take_last_id(s)
      let s = process_expr(s, rhs)
      let #(s, right) = take_last_id(s)
      add_node(s, Math(left, op, right))
    }
    syntax.Switch(high, low) -> {
      let #(s, condition) = take_last_id(s)
      let s = process_expr(s, high)
      let #(s, high) = take_last_id(s)
      let s = process_expr(s, low)
      let #(s, low) = take_last_id(s)
      add_node(s, Switch(condition, high, low))
    }
  }
}

fn resolve_link(vars: Vars, link: LazyLink) -> NodeId {
  case link {
    KnownLink(id) -> id
    LazyLink(var) -> resolve_var(vars, var, set.new())
  }
}

fn resolve_var(vars: Vars, var: String, seen: Set(String)) -> NodeId {
  let seen = case set.contains(seen, var) {
    True -> panic as { "`" <> var <> "` is defined recursively" }
    False -> set.insert(seen, var)
  }

  case dict.get(vars, var) {
    Ok(KnownLink(id)) -> id
    Ok(LazyLink(var)) -> resolve_var(vars, var, seen)
    Error(_) -> panic as { "`" <> var <> "` is not defined" }
  }
}
