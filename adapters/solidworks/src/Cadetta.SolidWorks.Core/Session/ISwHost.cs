namespace Cadetta.SolidWorks.Session
{
    /// <summary>
    /// Finds or launches the SOLIDWORKS application without naming the interop
    /// types, so attach-or-start is testable without SOLIDWORKS (adapters.md 2.2).
    /// </summary>
    /// <typeparam name="T">The application handle type (ISldWorks in the Com project).</typeparam>
    public interface ISwHost<T> where T : class
    {
        /// <summary>Returns the running instance, or null when none is running.</summary>
        T? TryAttach();

        /// <summary>Starts a new instance with no visible window and returns it.</summary>
        T StartHidden();

        /// <summary>Exits an instance this adapter started.</summary>
        void Quit(T app);
    }
}
