import argv
import gleam/bool
import gleam/io
import gleam/list
import gleam/result
import gleam/set
import gleave
import graph
import simplifile
import syntax

type Mixer =
  List(List(syntax.Node))

pub fn main() {
  case run() {
    Ok(_) -> Nil
    Error(err) -> {
      io.println_error(err)
      gleave.exit(1)
    }
  }
}

fn run() -> Result(Nil, String) {
  use path <- result.try(case argv.load().arguments {
    [path] -> Ok(path)
    _ -> Error("usage: ./vertx_mixer <file.mix>")
  })

  let assert Ok(raw) = simplifile.read(path)
  let assert Ok(tokens) = syntax.lex(raw)
  let assert Ok(mixer) = syntax.parse(tokens)

  use _ <- result.try(validate(mixer))

  let graph = graph.flatten(mixer)
  io.debug(graph)

  Ok(Nil)
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
