using System;
using Cadetta.SolidWorks.Push;
using Cadetta.SolidWorks.Save;

namespace Cadetta.SolidWorks.Session
{
    // frob:ticket 01M5496CQE8FPX6F7PD9Y6EMX0
    /// <summary>Opens a document for writing, so the RPC server can push without naming the SOLIDWORKS types.</summary>
    public interface IPushHost
    {
        /// <summary>Opens <paramref name="file"/> for a push; throws InvalidOperationException when it cannot be opened.</summary>
        IPushHandle Open(string file);
    }

    // frob:ticket 01M5496CQE8FPX6F7PD9Y6EMX0
    /// <summary>One document opened for a push; disposing closes it only when the adapter opened it.</summary>
    public interface IPushHandle : IDisposable
    {
        /// <summary>The document as the push service writes it.</summary>
        IPushDocument Document { get; }

        /// <summary>The document as the save step writes it to disk.</summary>
        ISaveTarget Target { get; }

        /// <summary>Records the push result on <see cref="Target"/> so the save step knows whether to write.</summary>
        void Settle(bool modified, bool rebuildFailed);
    }
}
