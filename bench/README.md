To run benchmarks, run `run_bench.sh` (in the repo root).

Files for benchmarking. Each directory is from a different source.

 - synthetic (artificial, hand-crafted examples)
 - json_org (https://json.org examples)
 - gh (GitHub API https://api.github.com/)
 - ncdc (NCDC NOAA API https://www.ncdc.noaa.gov/cdo-web/webservices/v2)
 - gov.uk (https://gov.uk)
 - penn (Penn Museum https://www.penn.museum/collections/objects/data.php)
 - doi (https://www.doi.org/factsheets/DOIProxy.html#rest-api)
 - penguin (http://www.penguinrandomhouse.biz/webservices/rest/)
 - rv (Rig Veda https://aninditabasu.github.io/indica/html/rv.html)
 - fda (https://open.fda.gov/apis/)

https://github.com/public-apis/public-apis is a useful meta-source.

We generate micro-benchmarks using `mk_micro.sh`, which will wipe out
and recreate the directory `micro`.
