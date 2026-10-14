namespace Cadetta.SolidWorks.Session
{
    /// <summary>An in-memory session that stands in for SOLIDWORKS in CI and tests.</summary>
    public sealed class FakeSwSession : ISwSession
    {
        /// <summary>Returns a fixed revision string.</summary>
        public string ProductVersion() => "fake";
    }
}
