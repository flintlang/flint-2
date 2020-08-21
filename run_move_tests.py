import re
from pathlib import Path
from typing import NamedTuple, Optional, List, Iterable
import subprocess, os, sys, shutil
from contextlib import contextmanager
import json


class CompilationError(Exception): ...


class FlintCompilationError(CompilationError): ...


class MoveRuntimeError(RuntimeError):
  def __init__(self, message, line=None):
    self.line = line
    super().__init__(message)

  @classmethod
  def from_output(cls, output):
    line = re.search(r"sub_status: Some\((\d+)\)", output)
    if line:
      line = int(line.group(1))
    return cls(output, line)


@contextmanager
def run_at_path(path):
  original_dir = os.getcwd()
  if isinstance(path, str):
    path = Path(path)
  os.chdir(path.expanduser())
  yield
  os.chdir(original_dir)


class Configuration(NamedTuple):
  libra_path: Path

  @classmethod
  def from_flint_config(cls):
    # add own path to .json file and libra
    with open(os.path.expanduser("flint_config.json")) as file:
      path = json.load(file).get("libraPath")  # .get defaults to None
      if path:  # If libraPath is defined and not empty
        return cls(path)
      else:
        return None


class Programme:
  path: Path
  config: Optional[Configuration]

  def __init__(self, path: Path, config: Optional[Configuration] = None):
    self.path = path
    self.config = config

  def contents(self):
    with open(self.path) as file:
      return file.read()

  @property
  def name(self):
    return self.path.stem


class MoveIRProgramme(Programme):
  temporary_test_path = Path("language/ir-testsuite/tests/flint2tests")

  def run(self):
    with run_at_path(self.config.libra_path):
      process = subprocess.Popen(
          ["cargo", "test", "-p", "ir-testsuite",
           "/".join(list(self.path.parts)[-2:])],
          stdout=subprocess.PIPE,
          stderr=subprocess.PIPE
      )
    output = process.stdout.read().decode(
        "utf8") + process.stderr.read().decode("utf8")
    if re.search(r"[^0\s]\s+failed", output) or not re.search(r"[1-9]\s+passed",
                                                              output):
      raise MoveRuntimeError.from_output(output)

  def with_testsuite(self, testsuite):
    assert isinstance(testsuite, MoveIRProgramme)
    new = TestRunner.default_behaviour_path / "temp" / self.path.name
    try:
      os.makedirs(TestRunner.default_behaviour_path / "temp")
    except FileExistsError:
      pass
    with open(new, "w") as file:
      testsuite_contents = testsuite.contents().split("//! provide module")
      for module in testsuite_contents[1:]:
        file.write(f"""
{module !s}
//! new-transaction
""")
      file.write(f"""\
{self.contents().replace("import 0x0.", "import Transaction.") !s}
//! new-transaction
{testsuite_contents[0] !s}
""")
    self.path = new

  def move_to_libra(self):
    testpath = self.config.libra_path / self.temporary_test_path / \
               self.path.parts[-1]
    try:
      os.makedirs(self.config.libra_path / self.temporary_test_path)
    except FileExistsError:
      pass
    else:
      print(f"Created new folder {testpath} for testfile")
    self.path.rename(testpath)
    self.path = testpath


class FlintProgramme(Programme):
  @property
  def using_stdlib(self):
    return "//! disable stdlib" not in self.contents()

  def compile(self) -> MoveIRProgramme:
    process = subprocess.Popen(["cargo", "run", "libra", str(self.path)],
                               stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE
                               )
    output = process.stdout.read() + process.stderr.read()
    if b"successfully wrote" not in output:
      raise FlintCompilationError(output.decode("utf8"))

    output_name = re.search(r"successfully wrote to (.+\.mvir)", str(output))
    if output_name:
      output_name = str(output_name.group(1))
    else:
      raise FlintCompilationError(output.decode("utf-8"))

    return MoveIRProgramme(Path(output_name), config=self.config)


class BehaviourTest(NamedTuple):
  programme: FlintProgramme
  testsuite: Optional[MoveIRProgramme] = None
  expected_fail_line: Optional[int] = None

  @classmethod
  def from_name(cls, name: str, *, config: Configuration):
    """
        Creates a behaviour test from a test name, searching for .flint and .mvir files
        The .flint file must exist, however the .mvir file is optional (it will just
        check compilation in such case).
        If you want your test to fail on an assertion on line x in Flint, you can write
        `//! expect fail x`, however, this expects the assertion to have that line number
        which may not be the case if the assertion is generated through fatalErrors or
        similar functions. It will work if a disallowed operation (type states, caller
        protections) has been attempted. Also note, only one fail is allowed per test.
        """

    move_path = TestRunner.default_behaviour_path / (name + ".mvir")
    move_programme = None
    expected_fail_line = None
    if move_path.exists():
      move_programme = MoveIRProgramme(move_path, config=config)
      expect_fail = re.search(r"//! expect fail (\d+)",
                              move_programme.contents(), flags=re.IGNORECASE)
      if expect_fail:
        expected_fail_line = int(expect_fail.group(1))

    return cls(
        FlintProgramme(
            TestRunner.default_behaviour_path / (name + ".flint"),
            config=config),
        move_programme,
        expected_fail_line
    )

  def test(self) -> bool:
    try:
      test = self.programme.compile()
    except FlintCompilationError as e:
      TestFormatter.behaviour_failed(self.programme.name,
                                     f"Flint Compilation Error: `{e !s}`")
      return False
    if self.testsuite:
      test.with_testsuite(self.testsuite)

    test.move_to_libra()

    try:
      test.run()
    except MoveRuntimeError as e:
      line, message = e.line or 0, f"Move Runtime Error: " \
                                   f"Error in {self.programme.path.name} line {e.line}: {e !s}"
    else:
      line = message = None
    if self.expected_fail_line != line:
      TestFormatter.behaviour_failed(self.programme.name,
                                     message or f"Move Missing Error: "
                                                f"No error raised in {self.programme.path.name} line {self.expected_fail_line}"
                                     )
      return False

    TestFormatter.behaviour_passed(self.programme.name)
    return True


class TestFormatter:
  FAIL = "\033[1;38;5;196m"
  SUCCESS = "\033[1;38;5;114m"
  END = "\033[m"

  @classmethod
  def behaviour_failed(cls, test, message):
    print(f"""\
{test}: {cls.FAIL}failed{cls.END}
\t{message}\
""")

  @classmethod
  def compilation_failed(cls, test, message):
    print(f"""\
{test}: {cls.FAIL}failed{cls.END}
\t{message}\
""")

  @classmethod
  def compilation_success(cls, test):
    print(f"Compilation test: {test}: {cls.SUCCESS}passed{cls.END}")

  @classmethod
  def behaviour_passed(cls, test):
    print(f"Behaviour test: {test}: {cls.SUCCESS}passed{cls.END}")

  @classmethod
  def all_failed(cls, tests: Iterable[str]):
    print(f"{cls.FAIL}Failed tests:{cls.END}")
    for test in tests:
      print(f"\t{cls.FAIL}{test}{cls.END}")

  @classmethod
  def complete_move(cls):
    print(f"\n\t{cls.SUCCESS}All MoveIR tests passed!{cls.END}\n")

  @classmethod
  def complete_compilation_checks(cls):
    print(
        f"\n\t{cls.SUCCESS}All compilation tests passed!{cls.END}\n")

  @classmethod
  def not_configured(cls):
    print("""\
MoveIR tests not yet configured on this computer
To run them please set "libraPath" in ~/.flint/flint_config.json to the root of the Libra directory\
""")


class TestRunner(NamedTuple):
  behaviour_tests: List[BehaviourTest]
  compilation_tests: List[FlintProgramme]
  default_behaviour_path = Path("tests/move_tests")
  default_compilation_test_path = Path("tests/compilation_tests")

  @classmethod
  def from_all(cls, names=[], config=None):
    behaviour_files = [file for file in
                       cls.default_behaviour_path.iterdir()
                       if file.suffix.endswith("flint")
                       if not names or file.stem in names]

    all_files = behaviour_files + [file for file in
                                   cls.default_compilation_test_path.iterdir()
                                   if file.suffix.endswith("flint")
                                   if not names or file.stem in names]

    return TestRunner([BehaviourTest.from_name(file.stem, config=config)
                       for file in behaviour_files],

                      [FlintProgramme(file, config=config)
                       for file in all_files])

  def run_behaviour_tests(self):
    passed = set()
    for test in self.behaviour_tests:
      try:
        if test.test():
          passed.add(test)
      except BaseException as e:
        print(f"Unexpected error `{e}`. Assuming failure")

    try:
      shutil.rmtree(
          MoveIRProgramme.config.libra_path / MoveIRProgramme.temporary_test_path)
      shutil.rmtree(self.default_behaviour_path / "temp")
    except:
      print(f"Could not remove temporary files")

    failed = set(self.behaviour_tests) - passed
    if failed:
      TestFormatter.all_failed(
          set(map(lambda t: t.programme.name, failed)))
      return 1
    else:
      TestFormatter.complete_move()
      return 0

  def run_compilation_tests(self):
    """
        Attempts to compile all tests in the compilation test folder. If you want
        a test to fail compilation, write somewhere in that file
        //! Fail compile <msg>
        where <msg> is a snippet of the error message that is expected does not
        have to be the whole thing
        """
    passed = set()
    for programme in self.compilation_tests:
      should_fail = False
      error_msg = re.search("//! compile fail ([\w `]+)",
                            programme.contents())
      if error_msg:
        print("Should fail", programme.name, error_msg)
        error_msg = error_msg.group(1)
        should_fail = True
      try:
        programme.compile()
        if should_fail:
          TestFormatter.compilation_failed(programme.name,
                                           "Did not fail to compile")
        else:
          TestFormatter.compilation_success(programme.name)
          passed.add(programme)

      except FlintCompilationError as e:
        if should_fail and error_msg in str(e):
          passed.add(programme)
          TestFormatter.compilation_success(programme.name)
        else:
          TestFormatter.compilation_failed(programme.name,
                                           "Failed to compile")

    failed = set(self.compilation_tests) - passed
    if failed:
      TestFormatter.all_failed(set(map(lambda t: t.name, failed)))
      return 1
    else:
      TestFormatter.complete_compilation_checks()
      return 0

  def run(self):
    self.run_behaviour_tests()
    self.run_compilation_tests()


if __name__ == '__main__':
  os.path.dirname(os.path.realpath(__file__))
  config = Configuration.from_flint_config()

  assert sys.argv[1] in ["all", "compilation", "behaviour"]

  if not config and sys.argv[1] != "compilation":
    TestFormatter.not_configured()
    sys.exit(0)

  test_runner = TestRunner.from_all(sys.argv[2:], config=config)

  # Run all, or run the given arguments (empty list is false)
  if sys.argv[1] == "all":
    sys.exit(test_runner.run())
  elif sys.argv[1] == "compilation":
    sys.exit(test_runner.run_compilation_tests())
  elif sys.argv[1] == "behaviour":
    sys.exit(test_runner.run_behaviour_tests())
  else:
    raise Exception(
        "Must specify 'all', 'compilation' or 'behaviour' as first program argument")
