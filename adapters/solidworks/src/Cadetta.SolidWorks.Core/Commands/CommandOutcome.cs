namespace Cadetta.SolidWorks.Commands
{
    // frob:ticket 01M4VB7VXXHJ6TKHF6J2STC1FE
    /// <summary>Result of one add-in command on a document: success or a user-presentable refusal (no exceptions for expected failures).</summary>
    public sealed class CommandOutcome
    {
        private CommandOutcome(bool ok, string message, string? code)
        {
            IsOk = ok;
            Message = message;
            Code = code;
        }

        /// <summary>True when the command did what was asked.</summary>
        public bool IsOk { get; }

        /// <summary>What happened, or why it was refused.</summary>
        public string Message { get; }

        /// <summary>Rule code of the refusal (R009, R010), or null.</summary>
        public string? Code { get; }

        /// <summary>A success.</summary>
        public static CommandOutcome Ok(string message) => new CommandOutcome(true, message, null);

        /// <summary>A refusal, optionally tagged with a rule code.</summary>
        public static CommandOutcome Refused(string message, string? code = null) => new CommandOutcome(false, message, code);
    }
}
