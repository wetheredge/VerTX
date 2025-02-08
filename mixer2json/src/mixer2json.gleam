import argv
import gleam/bool
import gleam/io
import gleam/list
import gleam/result
import gleam/set
import graph
import output
import simplifile
import syntax

type Mixer =
  List(List(syntax.Node))

pub fn main() {
  let path = case argv.load().arguments {
    [path] -> path
    _ -> panic as "usage: mixer2json <input.mixer>"
  }

  let mixer = case simplifile.read(path) {
    Ok(s) -> s
    Error(err) -> panic as simplifile.describe_error(err)
  }

  case convert(mixer) {
    Ok(json) -> io.println(json)
    Error(err) -> panic as err
  }
}

pub fn convert(raw: String) -> Result(String, String) {
  let assert Ok(tokens) = syntax.lex(raw)
  let assert Ok(mixer) = syntax.parse(tokens)

  use _ <- result.try(validate(mixer))

  let graph = graph.flatten(mixer)
  Ok(output.to_json(graph))
}

fn validate(mixer: Mixer) -> Result(Nil, String) {
  let all_sets = {
    use expr <- list.flat_map(mixer)
    use node <- list.filter_map(expr)
    case node {
      syntax.Set(var) -> Ok(var)
      _ -> Error(Nil)
    }
  }

  use vars <- result.try({
    use unique_vars, var <- list.try_fold(all_sets, set.new())
    use <- bool.lazy_guard(set.contains(unique_vars, var), fn() {
      Error("`" <> var <> "` is set multiple times")
    })
    Ok(set.insert(unique_vars, var))
  })

  // Check all gets match a set
  use _ <- result.try({
    use expr <- list.try_each(mixer)
    use node <- list.try_each(expr)
    case node {
      syntax.Get(var) ->
        case set.contains(vars, var) {
          True -> Ok(Nil)
          False -> Error("`" <> var <> "` is never set")
        }
      _ -> Ok(Nil)
    }
  })

  Ok(Nil)
}
