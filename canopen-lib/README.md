# CANOpen library


# Developer Notes

## Design Decisions

### Dealing with no-std

This crate is based on *tokio-socketcan* which is not available on no-std.
Thus, this library is developed in mind of easy refactoring towards no-std.
Any heap usage (like Box and trait objects) is not a primary design choice.

The rust type system is employed to a great degree for ease of code readability
and also to let the rust compiler do its job in finding type errors.

### Error handling

the std::error::Error trait is used for easy error propagation.
It is implicitly done by using the convenient *thisError* crate.


## Restructuring Ideas

Frame module

- each frame can be built
- each frame can be formatted
  - for monitoring)
- each frame can be parsed/converted from CANFrame
- if necessary a frame gets its sub-module like SDO
- each frame can be inspected
  - i.e. payload as enum
  - for filtering

## ToDo

- segmented tests for parsing !!!
- test it test segmented download
- clarify segmented download protocol (client and server command specifier at frames without index  )
- Refactor payload  with index . data as [u8, 4]

- no std varaint base on *embedded-can* hal api
  - [rust doc embedded-can](https://docs.rs/embedded-can/0.4.1/embedded_can/)
  - [crate embedded-can](https://crates.io/crates/embedded-can)
- buy shield with transceiver: https://www.waveshare.com/wiki/RS485_CAN_Shield to have can on mcu
- alternate: this board: https://www.olimex.com/Products/Duino/STM32/OLIMEXINO-STM32/open-source-hardware

### Example work in Segmented upload

- on node 0x12 = 18
- in sdo client and server as loopback device with bdd test

```
612.40.05.20.01.00.00.00.00 Upload request Object 2005 sub object 1. ccs: 0b010.0.0000: 010 -> initiate an upload

592.41.05.20.01.1A.00.00.00 It is segmented: Fine. It's 26 bytes long ccs: 41 -> 1 data size is set and segmented so size is in data field.

612.60.00.00.00.00.00.00.00  Upload segment req, Toggle = 0
592.00.54.69.6E.79.20.4E.6F  Upload segment resp, Toggle = 0
612.70.00.00.00.00.00.00.00  Upload segment req, Toggle = 1
592.10.64.65.20.2D.20.4D.65  Upload segment resp, Toggle = 1
612.60.00.00.00.00.00.00.00  Upload segment req, Toggle = 0
592.00.67.61.20.44.6F.6D.61  Upload segment resp, Toggle = 0
612.70.00.00.00.00.00.00.00  Upload segment req, Toggle = 1
592.15.69.6E.73.20.21.00.00  Last segment, 2 bytes free, Toggle = 1
```

## Block download

[source](https://github.com/christiansandberg/canopen/blob/master/canopen/sdo/client.py)

```python
class BlockUploadStream(io.RawIOBase):
    """File like object for reading from a variable using block upload."""

    #: Total size of data or ``None`` if not specified
    size = None

    blksize = 127

    crc_supported = False

    def __init__(self, sdo_client, index, subindex=0, request_crc_support=True):
        """
        :param canopen.sdo.SdoClient sdo_client:
            The SDO client to use for reading.
        :param int index:
            Object dictionary index to read from.
        :param int subindex:
            Object dictionary sub-index to read from.
        :param bool request_crc_support:
            If crc calculation should be requested when using block transfer            
        """
        self._done = False
        self.sdo_client = sdo_client
        self.pos = 0
        self._crc = sdo_client.crc_cls()
        self._server_crc = None
        self._ackseq = 0

        logger.debug("Reading 0x%X:%d from node %d", index, subindex,
                     sdo_client.rx_cobid - 0x600)
        # Initiate Block Upload
        request = bytearray(8)
        command = REQUEST_BLOCK_UPLOAD | INITIATE_BLOCK_TRANSFER
        if request_crc_support:
            command |= CRC_SUPPORTED
        struct.pack_into("<BHBBB", request, 0,
                         command, index, subindex, self.blksize, 0)
        response = sdo_client.request_response(request)
        res_command, res_index, res_subindex = SDO_STRUCT.unpack_from(response)
        if res_command & 0xE0 != RESPONSE_BLOCK_UPLOAD:
            raise SdoCommunicationError("Unexpected response 0x%02X" % res_command)
        # Check that the message is for us
        if res_index != index or res_subindex != subindex:
            raise SdoCommunicationError((
                "Node returned a value for 0x{:X}:{:d} instead, "
                "maybe there is another SDO client communicating "
                "on the same SDO channel?").format(res_index, res_subindex))
        if res_command & BLOCK_SIZE_SPECIFIED:
            self.size, = struct.unpack_from("<L", response, 4)
            logger.debug("Size is %d bytes", self.size)
        self.crc_supported = bool(res_command & CRC_SUPPORTED)
        # Start upload
        request = bytearray(8)
        request[0] = REQUEST_BLOCK_UPLOAD | START_BLOCK_UPLOAD
        sdo_client.send_request(request)

    def read(self, size=-1):
        """Read one segment which may be up to 7 bytes.
        :param int size:
            If size is -1, all data will be returned. Other values are ignored.
        :returns: 1 - 7 bytes of data or no bytes if EOF.
        :rtype: bytes
        """
        if self._done:
            return b""
        if size is None or size < 0:
            return self.readall()

        try:
            response = self.sdo_client.read_response()
        except SdoCommunicationError:
            response = self._retransmit()
        res_command, = struct.unpack_from("B", response)
        seqno = res_command & 0x7F
        if seqno == self._ackseq + 1:
            self._ackseq = seqno
        else:
            # Wrong sequence number
            response = self._retransmit()
            res_command, = struct.unpack_from("B", response)
        if self._ackseq >= self.blksize or res_command & NO_MORE_BLOCKS:
            self._ack_block()
        if res_command & NO_MORE_BLOCKS:
            n = self._end_upload()
            data = response[1:8 - n]
            self._done = True
        else:
            data = response[1:8]
        if self.crc_supported:
            self._crc.process(data)
            if self._done:
                if self._server_crc != self._crc.final():
                    self.sdo_client.abort(0x05040004)
                    raise SdoCommunicationError("CRC is not OK")
                logger.info("CRC is OK")
        self.pos += len(data)
        return data

    def _retransmit(self):
        logger.info("Only %d sequences were received. Requesting retransmission",
                    self._ackseq)
        end_time = time.time() + self.sdo_client.RESPONSE_TIMEOUT
        self._ack_block()
        while time.time() < end_time:
            response = self.sdo_client.read_response()
            res_command, = struct.unpack_from("B", response)
            seqno = res_command & 0x7F
            if seqno == self._ackseq + 1:
                # We should be back in sync
                self._ackseq = seqno
                return response
        raise SdoCommunicationError("Some data were lost and could not be retransmitted")

    def _ack_block(self):
        request = bytearray(8)
        request[0] = REQUEST_BLOCK_UPLOAD | BLOCK_TRANSFER_RESPONSE
        request[1] = self._ackseq
        request[2] = self.blksize
        self.sdo_client.send_request(request)
        if self._ackseq == self.blksize:
            self._ackseq = 0

    def _end_upload(self):
        response = self.sdo_client.read_response()
        res_command, self._server_crc = struct.unpack_from("<BH", response)
        if res_command & 0xE0 != RESPONSE_BLOCK_UPLOAD:
            self.sdo_client.abort(0x05040001)
            raise SdoCommunicationError("Unexpected response 0x%02X" % res_command)
        if res_command & 0x3 != END_BLOCK_TRANSFER:
            self.sdo_client.abort(0x05040001)
            raise SdoCommunicationError("Server did not end transfer as expected")
        # Return number of bytes not used in last message
        return (res_command >> 2) & 0x7

    def close(self):
        if self.closed:
            return
        super(BlockUploadStream, self).close()
        if self._done:
            request = bytearray(8)
            request[0] = REQUEST_BLOCK_UPLOAD | END_BLOCK_TRANSFER
            self.sdo_client.send_request(request)

    def tell(self):
        return self.pos

    def readinto(self, b):
        """
        Read bytes into a pre-allocated, writable bytes-like object b,
        and return the number of bytes read.
        """
        data = self.read(7)
        b[:len(data)] = data
        return len(data)

    def readable(self):
        return True


class BlockDownloadStream(io.RawIOBase):
    """File like object for block download."""

    def __init__(self, sdo_client, index, subindex=0, size=None, request_crc_support=True):
        """
        :param canopen.sdo.SdoClient sdo_client:
            The SDO client to use for communication.
        :param int index:
            Object dictionary index to read from.
        :param int subindex:
            Object dictionary sub-index to read from.
        :param int size:
            Size of data in number of bytes if known in advance.
        :param bool request_crc_support:
            If crc calculation should be requested when using block transfer            
        """
        self.sdo_client = sdo_client
        self.size = size
        self.pos = 0
        self._done = False
        self._seqno = 0
        self._crc = sdo_client.crc_cls()
        self._last_bytes_sent = 0
        self._current_block = []
        self._retransmitting = False
        command = REQUEST_BLOCK_DOWNLOAD | INITIATE_BLOCK_TRANSFER
        if request_crc_support:
            command |= CRC_SUPPORTED
        request = bytearray(8)
        logger.info("Initiating block download for 0x%X:%d", index, subindex)
        if size is not None:
            logger.debug("Expected size of data is %d bytes", size)
            command |= BLOCK_SIZE_SPECIFIED
            struct.pack_into("<L", request, 4, size)
        else:
            logger.warning("Data size has not been specified")
        SDO_STRUCT.pack_into(request, 0, command, index, subindex)
        response = sdo_client.request_response(request)
        res_command, res_index, res_subindex = SDO_STRUCT.unpack_from(response)
        if res_command & 0xE0 != RESPONSE_BLOCK_DOWNLOAD:
            self.sdo_client.abort(0x05040001)
            raise SdoCommunicationError(
                "Unexpected response 0x%02X" % res_command)
        # Check that the message is for us
        if res_index != index or res_subindex != subindex:
            self.sdo_client.abort()
            raise SdoCommunicationError((
                "Node returned a value for 0x{:X}:{:d} instead, "
                "maybe there is another SDO client communicating "
                "on the same SDO channel?").format(res_index, res_subindex))
        self._blksize, = struct.unpack_from("B", response, 4)
        logger.debug("Server requested a block size of %d", self._blksize)
        self.crc_supported = bool(res_command & CRC_SUPPORTED)

    def write(self, b):
        """
        Write the given bytes-like object, b, to the SDO server, and return the
        number of bytes written. This will be at most 7 bytes.
        :param bytes b:
            Data to be transmitted.
        :returns:
            Number of bytes successfully sent or ``None`` if length of data is
            less than 7 bytes and the total size has not been reached yet.
        """
        if self._done:
            raise RuntimeError("All expected data has already been transmitted")
        # Can send up to 7 bytes at a time
        data = b[0:7]
        if self.size is not None and self.pos + len(data) >= self.size:
            # This is the last data to be transmitted based on expected size
            self.send(data, end=True)
        elif len(data) < 7:
            # We can't send less than 7 bytes in the middle of a transmission
            return None
        else:
            self.send(data)
        return len(data)

    def send(self, b, end=False):
        """Send up to 7 bytes of data.
        :param bytes b:
            0 - 7 bytes of data to transmit.
        :param bool end:
            If this is the last data.
        """
        assert len(b) <= 7, "Max 7 bytes can be sent"
        if not end:
            assert len(b) == 7, "Less than 7 bytes only allowed if last data"
        self._seqno += 1
        command = self._seqno
        if end:
            command |= NO_MORE_BLOCKS
            self._done = True
            # Change expected ACK:ed sequence
            self._blksize = self._seqno
            # Save how many bytes this message contains since this is the last
            self._last_bytes_sent = len(b)
        request = bytearray(8)
        request[0] = command
        request[1:len(b) + 1] = b
        self.sdo_client.send_request(request)
        self.pos += len(b)
        # Add the sent data to the current block buffer
        self._current_block.append(b)
        # Don't calculate crc if retransmitting
        if self.crc_supported and not self._retransmitting:
            # Calculate CRC
            self._crc.process(b)
        if self._seqno >= self._blksize:
            # End of this block, wait for ACK
            self._block_ack()

    def tell(self):
        return self.pos

    def _block_ack(self):
        logger.debug("Waiting for acknowledgement of last block...")
        response = self.sdo_client.read_response()
        res_command, ackseq, blksize = struct.unpack_from("BBB", response)
        if res_command & 0xE0 != RESPONSE_BLOCK_DOWNLOAD:
            self.sdo_client.abort(0x05040001)
            raise SdoCommunicationError(
                "Unexpected response 0x%02X" % res_command)
        if res_command & 0x3 != BLOCK_TRANSFER_RESPONSE:
            self.sdo_client.abort(0x05040001)
            raise SdoCommunicationError("Server did not respond with a "
                                        "block download response")
        if ackseq != self._blksize:
            # Sequence error, try to retransmit
            self._retransmit(ackseq, blksize)
            # We should be back in sync
            return
        # Clear the current block buffer
        self._current_block = []
        logger.debug("All %d sequences were received successfully", ackseq)
        logger.debug("Server requested a block size of %d", blksize)
        self._blksize = blksize
        self._seqno = 0
        
    def _retransmit(self, ackseq, blksize):
        """Retransmit the failed block"""
        logger.info(("%d of %d sequences were received. "
                 "Will start retransmission") % (ackseq, self._blksize))
        # Sub blocks betwen ackseq and end of corrupted block need to be resent
        # Get the part of the block to resend
        block = self._current_block[ackseq:]
        # Go back to correct position in stream
        self.pos = self.pos - (len(block) * 7)
        # Reset the _current_block before starting the retransmission
        self._current_block = []
        # Reset _seqno and update blksize
        self._seqno = 0
        self._blksize = blksize
        # We are retransmitting
        self._retransmitting = True
        # Resend the block
        for b in block:
            self.write(b)
        self._retransmitting = False

    def close(self):
        """Closes the stream."""
        if self.closed:
            return
        super(BlockDownloadStream, self).close()
        if not self._done:
            logger.error("Block transfer was not finished")
        command = REQUEST_BLOCK_DOWNLOAD | END_BLOCK_TRANSFER
        # Specify number of bytes in last message that did not contain data
        command |= (7 - self._last_bytes_sent) << 2
        request = bytearray(8)
        request[0] = command
        if self.crc_supported:
            # Add CRC
            struct.pack_into("<H", request, 1, self._crc.final())
        logger.debug("Ending block transfer...")
        response = self.sdo_client.request_response(request)
        res_command, = struct.unpack_from("B", response)
        if not res_command & END_BLOCK_TRANSFER:
            raise SdoCommunicationError("Block download unsuccessful")
        logger.info("Block download successful")

    def writable(self):
        return True
Footer
```


# References

- https://github.com/CANopenNode/CANopenDemo (demo implemnetation linux in C)
- https://github.com/CANopenNode/CANopenNode (node implementation linux in C)
- 304 - safety related protocol


