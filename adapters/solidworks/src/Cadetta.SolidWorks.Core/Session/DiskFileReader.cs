using System.IO;
using Cadetta.SolidWorks.Save;

namespace Cadetta.SolidWorks.Session
{
    // frob:ticket 01M5496CQE8FPX6F7PD9Y6EMX0
    /// <summary>IFileReader over the real file system, for digests of saved files.</summary>
    public sealed class DiskFileReader : IFileReader
    {
        /// <summary>Returns the full contents of <paramref name="path"/>.</summary>
        public byte[] ReadAllBytes(string path) => File.ReadAllBytes(path);
    }
}
