# Benchmark TODO

## Speed things up

- Wait for mounts by spinning on `mountpoint -q`
- Shorten `sleep` times

## Workloads not yet implemented

- **read_k.sh**: Read k files out of N >> k (needs parametric workload support in bench.sh)
- **find_commits.sh**: Find commits matching a pattern from a GH dump
- **modify_graphql.sh**: Modify a GraphQL call and write it back (requires a GraphQL fixture)

## Format conversion benchmarks

- Run bench with `-t yaml` (input JSON, output YAML) to exercise the cross-format code path
- Compare same-type vs different-type serialization timing in the `saving` activity

## pack/unpack benchmarks

- Benchmark `pack` and `unpack` as standalone tools (separate script, e.g. `bench_pack.sh`)
- Compare ffs mount+workload total time vs pack-then-shell-op-then-unpack roundtrip (needs fresh plots)

## Data / fixtures needed

- GitHub API dump with commit history (for commit search workload)
- GraphQL fixture file (for modify_graphql workload)
