// frob:ticket 01M4VB7VTPQEHBVQ8ZCR2T1G5K
namespace Cadetta.SolidWorks.AddIn.Client
{
    /// <summary>Outcome of a CLI call: a value or a user-presentable error (no exceptions for expected failures).</summary>
    public sealed class CliResult<T> where T : class
    {
        private CliResult(T? value, string? error)
        {
            Value = value;
            Error = error;
        }

        /// <summary>The value when the call succeeded, else null.</summary>
        public T? Value { get; }

        /// <summary>The error text when the call failed, else null.</summary>
        public string? Error { get; }

        /// <summary>True when the call succeeded.</summary>
        public bool IsOk => Value != null;

        /// <summary>A successful result.</summary>
        public static CliResult<T> Ok(T value) => new CliResult<T>(value, null);

        /// <summary>A failed result.</summary>
        public static CliResult<T> Fail(string error) => new CliResult<T>(null, error);
    }
}
