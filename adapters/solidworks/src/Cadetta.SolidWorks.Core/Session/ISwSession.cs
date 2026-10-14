namespace Cadetta.SolidWorks.Session
{
    /// <summary>
    /// Every SOLIDWORKS call goes through this interface so everything above it
    /// is testable without SOLIDWORKS (solidworks.md 1).
    /// </summary>
    public interface ISwSession
    {
        /// <summary>The SOLIDWORKS revision string, for logging and diagnostics.</summary>
        string ProductVersion();
    }
}
