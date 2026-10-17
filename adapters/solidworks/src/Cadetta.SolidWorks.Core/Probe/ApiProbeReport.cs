using System.Collections.Generic;
using System.Text;

namespace Cadetta.SolidWorks.Probe
{
    // frob:ticket 01M4VH6XQF4GRQM20G0Z9SWC8W
    /// <summary>Renders probe results as the markdown table in API-SURFACE.md.</summary>
    public static class ApiProbeReport
    {
        /// <summary>One table row per result; pipes and newlines in cells are escaped.</summary>
        public static string ToMarkdown(IEnumerable<ApiProbeResult> results)
        {
            var sb = new StringBuilder();
            sb.Append("| API | Call | Result | Detail |\n|---|---|---|---|\n");
            foreach (var r in results)
            {
                sb.Append("| ").Append(Cell(r.Api)).Append(" | `").Append(Cell(r.Call)).Append("` | ")
                  .Append(r.Ok ? "ok" : "FAIL").Append(" | ").Append(Cell(r.Detail)).Append(" |\n");
            }
            return sb.ToString();
        }

        /// <summary>True when every result is ok.</summary>
        public static bool AllOk(IEnumerable<ApiProbeResult> results)
        {
            foreach (var r in results)
            {
                if (!r.Ok)
                {
                    return false;
                }
            }
            return true;
        }

        private static string Cell(string s) =>
            s.Replace("|", "\\|").Replace("\r", " ").Replace("\n", " ");
    }
}
