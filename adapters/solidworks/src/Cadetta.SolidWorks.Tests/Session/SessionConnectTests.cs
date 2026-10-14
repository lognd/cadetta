using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Session;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Session
{
    public class SessionConnectTests
    {
        private sealed class App
        {
        }

        private sealed class FakeHost : ISwHost<App>
        {
            private readonly App? _running;

            public FakeHost(App? running)
            {
                _running = running;
            }

            public int Starts { get; private set; }
            public int Quits { get; private set; }

            public App? TryAttach() => _running;

            public App StartHidden()
            {
                Starts++;
                return new App();
            }

            public void Quit(App app)
            {
                Quits++;
            }
        }

        [Fact]
        public void Running_instance_is_attached_and_not_started()
        {
            var running = new App();
            var host = new FakeHost(running);
            using (var conn = SwConnection<App>.Connect(host, Log.Null()))
            {
                Assert.Same(running, conn.App);
                Assert.False(conn.Started);
            }
            Assert.Equal(0, host.Starts);
        }

        [Fact]
        public void Missing_instance_is_started_hidden()
        {
            var host = new FakeHost(null);
            using (var conn = SwConnection<App>.Connect(host, Log.Null()))
            {
                Assert.True(conn.Started);
                Assert.NotNull(conn.App);
            }
            Assert.Equal(1, host.Starts);
        }

        [Fact]
        public void Dispose_quits_only_an_instance_the_adapter_started()
        {
            var attached = new FakeHost(new App());
            SwConnection<App>.Connect(attached, Log.Null()).Dispose();
            Assert.Equal(0, attached.Quits);

            var started = new FakeHost(null);
            var conn = SwConnection<App>.Connect(started, Log.Null());
            conn.Dispose();
            conn.Dispose();
            Assert.Equal(1, started.Quits);
        }
    }
}
