{ pkgs }:
let
  fixture = pkgs.runCommand "sqlite-callgrind-fixture.db" {
    nativeBuildInputs = [ pkgs.sqlite ];
  } ''
    export LC_ALL=C TZ=UTC
    sqlite3 "$out" < ${./fixture.sql}
    test "$(sqlite3 "$out" 'PRAGMA integrity_check;')" = ok
  '';
in {
  # Registry: each entry becomes profiles-NAME, smoke-NAME and integration-NAME.
  # argv names the actual profiled binary; shell wrappers would profile a shell.
  sqlite = {
    inherit fixture;
    plan = {
      schema = 1;
      name = "sqlite";
      variants = map (pages: {
        name = "sqlite-cache-${toString pages}";
        argv = [ "${pkgs.sqlite}/bin/sqlite3" "${fixture}" ];
        stdin = toString (pkgs.writeText "sqlite-cache-${toString pages}.sql"
          (builtins.replaceStrings [ "@SQLITE_CACHE_PAGES@" ] [ (toString pages) ]
            (builtins.readFile ./workload.sql.in)));
      }) [ 64 4096 ];
    };
  };
}
