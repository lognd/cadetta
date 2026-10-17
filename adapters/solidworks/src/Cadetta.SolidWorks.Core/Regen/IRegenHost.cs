using System;
using System.Collections.Generic;

namespace Cadetta.SolidWorks.Regen
{
    // frob:ticket 01M4SV2TVM50WJRZ42ZHXVD96N
    /// <summary>One feature that failed to rebuild, as SOLIDWORKS reports it.</summary>
    public sealed class RebuildError
    {
        /// <summary>The feature's name in the feature tree (empty when the failure names no feature).</summary>
        public string Feature { get; set; } = "";

        /// <summary>The native error code (swFeatureError_e), or 0 when unknown.</summary>
        public int Code { get; set; }

        /// <summary>A short description of the failure.</summary>
        public string Message { get; set; } = "";
    }

    // frob:ticket 01M4SV2TVM50WJRZ42ZHXVD96N
    /// <summary>One open document seen through what a regen needs.</summary>
    public interface IRegenDocument
    {
        /// <summary>Path of the document file.</summary>
        string Path { get; }

        /// <summary>True when the document cannot be written (read-only or checked out elsewhere): R009.</summary>
        bool ReadOnly { get; }

        /// <summary>True when the document holds unsaved edits the adapter did not make: R009 (OQ-8).</summary>
        bool UserDirty { get; }

        /// <summary>Forces a full rebuild and returns the features that failed; empty when it rebuilt cleanly.</summary>
        IReadOnlyList<RebuildError> Rebuild();
    }

    // frob:ticket 01M4SV2TVM50WJRZ42ZHXVD96N
    /// <summary>Opens documents for a regen, so the RPC server can regenerate without naming the SOLIDWORKS types.</summary>
    public interface IRegenHost
    {
        /// <summary>Opens <paramref name="file"/> for a regen; throws InvalidOperationException when it cannot be opened.</summary>
        IRegenHandle Open(string file);
    }

    // frob:ticket 01M4SV2TVM50WJRZ42ZHXVD96N
    /// <summary>One document opened for a regen; disposing closes it only when the adapter opened it.</summary>
    public interface IRegenHandle : IDisposable
    {
        /// <summary>The document as the regen service rebuilds it.</summary>
        IRegenDocument Document { get; }
    }
}
