namespace Cadetta.SolidWorks.Save
{
    /// <summary>One document of a drive session, as far as saving is concerned.</summary>
    public interface ISaveTarget
    {
        /// <summary>Path of the document file.</summary>
        string Path { get; }

        /// <summary>True when the adapter changed the document in this session.</summary>
        bool Modified { get; }

        /// <summary>True when the last rebuild of the document failed (R004).</summary>
        bool RebuildFailed { get; }

        /// <summary>Writes the document to disk; returns an error message, or null on success.</summary>
        string? Save();
    }

    /// <summary>Reads the bytes of a saved file, so the digest covers what is on disk.</summary>
    public interface IFileReader
    {
        /// <summary>Returns the full contents of <paramref name="path"/>.</summary>
        byte[] ReadAllBytes(string path);
    }
}
