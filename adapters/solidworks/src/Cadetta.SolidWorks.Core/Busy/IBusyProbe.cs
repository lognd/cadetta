namespace Cadetta.SolidWorks.Busy
{
    /// <summary>Reports whether the interactive session is in a state where writing would corrupt it (R009).</summary>
    public interface IBusyProbe
    {
        /// <summary>True when a modal dialog is open.</summary>
        bool ModalDialogOpen();

        /// <summary>True when a command (for example a sketch edit or feature dialog) is active.</summary>
        bool CommandActive();
    }
}
