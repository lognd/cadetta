using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Busy
{
    /// <summary>Pre-push check that refuses to touch a session with a modal dialog or active command (R009, adapters.md 2.3).</summary>
    public static class BusyGuard
    {
        /// <summary>The diagnostic code for a refused push.</summary>
        public const string Code = "R009";

        /// <summary>Returns null when the session is idle, else the R009 message explaining why the push is refused.</summary>
        public static string? Check(IBusyProbe probe, Log log)
        {
            if (probe.ModalDialogOpen())
            {
                log.Warn("R009: a modal dialog is open; refusing to push");
                return "a modal dialog is open in SOLIDWORKS; close it and retry";
            }
            if (probe.CommandActive())
            {
                log.Warn("R009: a command is active; refusing to push");
                return "a command is active in SOLIDWORKS; finish or cancel it and retry";
            }
            log.Debug("session is idle; push may proceed");
            return null;
        }
    }
}
