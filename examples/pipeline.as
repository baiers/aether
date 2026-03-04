// Aether-Short (AS) Demo Pipeline
// This is ~60% fewer lines than equivalent full .ae syntax.
// Run:          aether examples/pipeline.as
// Inspect .ae:  aether examples/pipeline.as --expand-only

@pipeline 0xFF_DEMO

  ::CTX {
    $0xSEED: 42
  }

  // Generate a dataset — note $0xSEED declared as arg so executor binds it
  $0xDATA: JSON = @std.proc.transform($0xSEED) {
    import random
    random.seed($0xSEED)
    return [round(random.gauss(100, 15), 2) for _ in range(50)]
  }

  // Compute mean and std deviation
  $0xSTATS: JSON = @std.math.stats($0xDATA) {
    data = $0xDATA
    n = len(data)
    mean = sum(data) / n
    variance = sum((x - mean) ** 2 for x in data) / n
    return {"n": n, "mean": round(mean, 2), "std": round(variance ** 0.5, 2)}
  } | ASSERT $0xSTATS["n"] == 50 OR HALT

  // Find min / max / median
  $0xRANGE: JSON = @std.math.stats($0xDATA) {
    data = sorted($0xDATA)
    n = len(data)
    median = data[n // 2] if n % 2 else (data[n // 2 - 1] + data[n // 2]) / 2
    return {"min": round(data[0], 2), "max": round(data[-1], 2), "median": round(median, 2)}
  }

  // Summarize both results into a report string
  $0xREPORT: JSON = @std.proc.transform($0xSTATS, $0xRANGE) {
    import json
    stats = $0xSTATS
    rng = $0xRANGE
    summary = "n=" + str(stats["n"]) + " mean=" + str(stats["mean"]) + " std=" + str(stats["std"]) + " | min=" + str(rng["min"]) + " median=" + str(rng["median"]) + " max=" + str(rng["max"])
    return {"summary": summary}
  }

@end
