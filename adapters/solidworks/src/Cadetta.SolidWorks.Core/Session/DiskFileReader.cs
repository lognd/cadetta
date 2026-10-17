using System.IO;
using Cadetta.SolidWorks.Save;

namespace Cadetta.SolidWorks.Session
{
    // frob:ticket 01M5496CQE8FPX6F7PD9Y6EMX0
    /// <summary>IFileReader over the real file system, for digests of saved files.</summary>
    public sealed class DiskFileReader : IFileReader
    {
        /// <summary>Returns the full contents of <paramref name="path"/>, sharing the file with SOLIDWORKS which keeps its documents open (frob:ticket ~NEHY9X7).</summary>
        public byte[] ReadAllBytes(string path)
        {
            using (var fs = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete))
            using (var ms = new MemoryStream())
            {
                fs.CopyTo(ms);
                return ms.ToArray();
            }
        }
    }
}
