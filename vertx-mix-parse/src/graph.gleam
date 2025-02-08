import gleam/dict.{type Dict}
import gleam/io
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/set.{type Set}
import syntax

pub type Graph {
  Graph(nodes: List(Node(NodeId)), outputs: Dict(Int, NodeId))
}

pub type NodeId =
  Int

pub type Node(link) {
  Input(name: String)
  Const(value: Int)
  Output(channel: Int, input: link)
  Switch(condition: link, high: link, low: link)
}

type LazyLink {
  KnownLink(NodeId)
  LazyLink(var: String)
}

type State {
  State(
    nodes: List(Node(LazyLink)),
    outputs: Dict(Int, LazyLink),
    vars: Vars,
    last_id: Option(LazyLink),
  )
}

type Vars =
  Dict(String, LazyLink)

fn add_node(s: State, node: Node(LazyLink)) -> State {
  State(
    ..s,
    nodes: [node, ..s.nodes],
    last_id: Some(KnownLink(list.length(s.nodes))),
  )
}

fn last_id(s: State) -> LazyLink {
  let assert Some(last_id) = s.last_id
  last_id
}

pub fn flatten(mixer: List(syntax.Expr)) -> Graph {
  let result = {
    use s, expr <- list.fold(mixer, State([], dict.new(), dict.new(), None))
    process_expr(s, expr)
  }

  let vars = result.vars

  let nodes =
    list.reverse(result.nodes)
    |> list.map(fn(node) {
      case node {
        Const(value) -> Const(value)
        Input(name) -> Input(name)
        Output(channel, input) -> Output(channel, resolve_link(vars, input))
        Switch(condition, high, low) ->
          Switch(
            resolve_link(vars, condition),
            resolve_link(vars, high),
            resolve_link(vars, low),
          )
      }
    })

  let outputs =
    dict.map_values(result.outputs, fn(_, link) { resolve_link(vars, link) })

  Graph(nodes, outputs)
}

fn process_expr(s: State, expr: syntax.Expr) -> State {
  let s = State(..s, last_id: None)
  use s, node <- list.fold(expr, s)
  case node {
    syntax.Const(value) -> add_node(s, Const(value))
    syntax.Get(var) -> State(..s, last_id: Some(LazyLink(var)))
    syntax.Input(name) -> add_node(s, Input(name))
    syntax.Output(channel) ->
      State(
        ..s,
        outputs: dict.insert(s.outputs, channel, last_id(s)),
        last_id: None,
      )
    syntax.Set(var) ->
      State(..s, vars: dict.insert(s.vars, var, last_id(s)), last_id: None)
    syntax.Switch(high, low) -> {
      let condition = last_id(s)
      let s = process_expr(s, high)
      let high = last_id(s)
      let s = process_expr(s, low)
      let low = last_id(s)
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
    True -> {
      io.println_error("`" <> var <> "` is defined recursively")
      panic
    }
    False -> set.insert(seen, var)
  }

  case dict.get(vars, var) {
    Ok(KnownLink(id)) -> id
    Ok(LazyLink(var)) -> resolve_var(vars, var, seen)
    Error(_) -> {
      io.println_error("`" <> var <> "` is not defined")
      panic
    }
  }
}
