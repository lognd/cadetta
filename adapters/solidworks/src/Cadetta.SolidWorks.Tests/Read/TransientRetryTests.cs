using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Read;
using Xunit;

namespace Cadetta.SolidWorks.Tests.Read
{
    public class TransientRetryTests
    {
        /// frob:tests Cadetta.SolidWorks.Read.TransientRetry.Run
        [Fact]
        public void Server_fault_once_then_success_returns_value()
        {
            var calls = 0;
            var waits = new List<TimeSpan>();
            var v = TransientRetry.Run("t", Log.Null(), () =>
            {
                if (++calls == 1) throw new COMException("fault", TransientRetry.ServerFault);
                return 7;
            }, waits.Add);
            Assert.Equal(7, v);
            Assert.Equal(2, calls);
            Assert.Single(waits);
        }

        /// frob:tests Cadetta.SolidWorks.Read.TransientRetry.Run
        [Fact]
        public void Persistent_fault_gives_up_after_max_attempts()
        {
            var calls = 0;
            Assert.Throws<COMException>(() => TransientRetry.Run<int>("t", Log.Null(),
                () => { calls++; throw new COMException("fault", TransientRetry.ServerFault); }, _ => { }));
            Assert.Equal(TransientRetry.MaxAttempts, calls);
        }

        /// frob:tests Cadetta.SolidWorks.Read.TransientRetry.Run
        [Fact]
        public void Non_transient_error_is_not_retried()
        {
            var calls = 0;
            Assert.Throws<COMException>(() => TransientRetry.Run<int>("t", Log.Null(),
                () => { calls++; throw new COMException("other", unchecked((int)0x80004005)); }, _ => { }));
            Assert.Equal(1, calls);
        }
    }
}
