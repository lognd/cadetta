using Cadetta.SolidWorks.Busy;
using Cadetta.SolidWorks.Logging;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Busy
{
    public class BusyGuardTests
    {
        private sealed class Probe : IBusyProbe
        {
            public bool Modal;
            public bool Command;
            public bool ModalDialogOpen() => Modal;
            public bool CommandActive() => Command;
        }

        /// <summary>Mimics the push step: only runs when the guard allows it.</summary>
        private static int PushIfIdle(IBusyProbe probe, out string? refusal)
        {
            refusal = BusyGuard.Check(probe, Log.Null());
            return refusal == null ? 1 : 0;
        }

        [Fact]
        public void Modal_dialog_refuses_push_with_r009()
        {
            var pushes = PushIfIdle(new Probe { Modal = true }, out var refusal);
            Assert.Equal(0, pushes);
            Assert.Contains("modal dialog", refusal);
            Assert.Equal("R009", BusyGuard.Code);
        }

        [Fact]
        public void Active_command_refuses_push_with_r009()
        {
            var pushes = PushIfIdle(new Probe { Command = true }, out var refusal);
            Assert.Equal(0, pushes);
            Assert.Contains("command is active", refusal);
        }

        [Fact]
        public void Idle_session_allows_push()
        {
            var pushes = PushIfIdle(new Probe(), out var refusal);
            Assert.Equal(1, pushes);
            Assert.Null(refusal);
        }
    }
}
