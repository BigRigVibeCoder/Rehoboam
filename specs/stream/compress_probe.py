"""Probe: bytes per event for a synthetic channel-A stream in Parquet+zstd (an upper bound: synthetic IDs are less
regular than real ones). Columns mirror the World Stream EVENT record."""
import duckdb, os, sys
out = sys.argv[1]
con = duckdb.connect()
N = 1_000_000
con.execute(f"""
CREATE TABLE ev AS
WITH base AS (SELECT i, hash(i) AS h1, hash(i*7919+1) AS h2, hash(i*104729+3) AS h3 FROM range({N}) t(i))
SELECT
  i::UBIGINT                                    AS event_id,
  (i * 120)::BIGINT + (h1 % 120)::BIGINT        AS tick,          -- ~monotone sim-time
  (floor(pow((h1 % 10000)/10000.0, 3) * 40))::UTINYINT AS act_class, -- Zipf-ish over 40 classes
  (floor(pow((h2 % 1000000)/1000000.0, 2) * 100000))::UINTEGER AS actor, -- skewed over 100k persons
  CASE WHEN h3 % 10 < 4 THEN (floor(pow((h3 % 1000000)/1000000.0, 2) * 100000))::UINTEGER END AS target,
  (floor(pow((h2 % 7777)/7777.0, 2) * 2000))::USMALLINT AS place,
  CASE WHEN h1 % 3 = 0 AND i > 60 THEN (i - 1 - (h3 % 50)::BIGINT)::UBIGINT END AS cause_event,  -- edge to a recent event
  (h2 % 3)::UTINYINT                            AS cause_kind,
  (h3 % 256)::UTINYINT                          AS significance
FROM base
""")
for codec in ["zstd", "snappy"]:
    p = f"{out}/ev_{codec}.parquet"
    con.execute(f"COPY ev TO '{p}' (FORMAT parquet, COMPRESSION {codec}, ROW_GROUP_SIZE 122880)")
    print(codec, round(os.path.getsize(p)/N, 1), "bytes/event")
raw = N * (8+8+1+4+4+2+8+1+1)
print("raw fixed-width", round(raw/N,1), "bytes/event")
