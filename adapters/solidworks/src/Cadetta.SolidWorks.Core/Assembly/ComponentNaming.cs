using System.Text.RegularExpressions;

namespace Cadetta.SolidWorks.Assembly
{
    /// <summary>
    /// Spellings of an assembly component: the feature-tree instance name
    /// `wheel<2>` carried in a locator's `component` field, and the file it
    /// instantiates (adapters.md 3.5; emitters/sw-equations.md 4).
    /// frob:ticket 01M4SWME04D18722NZS7TE82RN
    /// </summary>
    public static class ComponentNaming
    {
        private static readonly Regex Name2 = new Regex(@"^(?<name>.+)-(?<n>\d+)$", RegexOptions.Compiled);

        /// <summary>`Sub-1/Bracket-2` to `Bracket<2>`; null when the name has no instance suffix.</summary>
        public static string? Instance(string name2)
        {
            var leaf = name2.Substring(name2.LastIndexOf('/') + 1);
            var m = Name2.Match(leaf);
            return m.Success ? m.Groups["name"].Value + "<" + m.Groups["n"].Value + ">" : null;
        }

        /// <summary>The file name of a document path, either slash style.</summary>
        public static string FileOf(string path) => System.IO.Path.GetFileName(path.Replace('\\', '/'));
    }
}
