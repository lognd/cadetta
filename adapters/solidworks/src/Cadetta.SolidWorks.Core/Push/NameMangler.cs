using System.Text;

namespace Cadetta.SolidWorks.Push
{
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    /// <summary>Created native names (adapters.md 3.1): a pure function of the canonical path and the prefix.</summary>
    public static class NameMangler
    {
        /// <summary>Drops package and modules, turns `.` and `/` into `__`, `[i]` into `_i`, drops `[*]`, and prepends <paramref name="prefix"/>.</summary>
        public static string Mangle(string path, string prefix = "")
        {
            var cut = path.LastIndexOf("::", System.StringComparison.Ordinal);
            var rest = cut >= 0 ? path.Substring(cut + 2) : path;
            var sb = new StringBuilder(prefix);
            for (var i = 0; i < rest.Length; i++)
            {
                var c = rest[i];
                if (c == '[')
                {
                    var end = rest.IndexOf(']', i);
                    if (end < 0) { sb.Append('_'); continue; }
                    var idx = rest.Substring(i + 1, end - i - 1);
                    if (idx != "*") sb.Append('_').Append(idx);
                    i = end;
                }
                else if (c == '.' || c == '/')
                {
                    sb.Append("__");
                }
                else
                {
                    sb.Append(c);
                }
            }
            return sb.ToString();
        }
    }
}
