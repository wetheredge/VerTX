import gleeunit
import gleeunit/should
import mixer2json

pub fn main() {
  gleeunit.main()
}

pub fn convert_3d_plane_test() {
  let mixer =
    "
    input(s1) -> set(armed);

    # if !armed, zero throttle
    get(armed) -> switch(input(throttle), const(0)) -> output(1);
    input(roll) -> output(2);
    input(pitch) -> output(3);
    input(yaw) -> output(4);
    get(armed) -> output(5);"
  let json =
    "{\"nodes\":[{\"type\":\"input\",\"name\":\"s1\"},{\"type\":\"input\",\"name\":\"throttle\"},{\"type\":\"const\",\"value\":0},{\"type\":\"switch\",\"condition\":0,\"high\":1,\"low\":2},{\"type\":\"input\",\"name\":\"roll\"},{\"type\":\"input\",\"name\":\"pitch\"},{\"type\":\"input\",\"name\":\"yaw\"}],\"outputs\":{\"1\":3,\"2\":4,\"3\":5,\"4\":6,\"5\":0}}"

  mixer2json.convert(mixer)
  |> should.equal(Ok(json))
}

pub fn convert_simple_quad_test() {
  let mixer =
    "
    input(throttle) -> output(1);
    input(roll) -> output(2);
    input(pitch) -> output(3);
    input(yaw) -> output(4);
    input(s1) -> output(5);
    input(s2) -> output(6);"
  let json =
    "{\"nodes\":[{\"type\":\"input\",\"name\":\"throttle\"},{\"type\":\"input\",\"name\":\"roll\"},{\"type\":\"input\",\"name\":\"pitch\"},{\"type\":\"input\",\"name\":\"yaw\"},{\"type\":\"input\",\"name\":\"s1\"},{\"type\":\"input\",\"name\":\"s2\"}],\"outputs\":{\"1\":0,\"2\":1,\"3\":2,\"4\":3,\"5\":4,\"6\":5}}"

  mixer2json.convert(mixer)
  |> should.equal(Ok(json))
}
