using Cadetta.SolidWorks.Busy;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Push;

namespace Cadetta.SolidWorks.Commands
{
    // frob:ticket 01M4VB7VXXHJ6TKHF6J2STC1FE
    /// <summary>Refuses an add-in write on a document a running sync could own or the user is editing (R009, adapters.md 2.2).</summary>
    public static class DocumentGate
    {
        /// <summary>The refusal code shared with the push path.</summary>
        public const string Code = BusyGuard.Code;

        /// <summary>Returns null when <paramref name="doc"/> may be written, else the R009 refusal; a held sync lock refuses too.</summary>
        public static CommandOutcome? Check(IPushDocument doc, IBusyProbe probe, bool syncRunning, Log log)
        {
            if (syncRunning)
            {
                log.Warn("R009: a sync holds " + doc.Path + "; refusing the command");
                return CommandOutcome.Refused("a sync is running on this document; wait for it to finish", Code);
            }
            if (doc.ReadOnly)
            {
                log.Warn("R009: " + doc.Path + " is read-only");
                return CommandOutcome.Refused("the document is read-only or checked out elsewhere", Code);
            }
            var busy = BusyGuard.Check(probe, log);
            if (busy != null)
            {
                return CommandOutcome.Refused(busy, Code);
            }
            return null;
        }
    }
}
