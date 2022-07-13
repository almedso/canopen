Feature: Illustrate CANOpen step examples

  Rule: Heartbeat

    Scenario: Heartbeat is sent at given
      Given Some arbitrary text ... Node 0x1A is up
      When Wait 10 ms ... some arbitrary text
      Then Wait 10 ms ... some arbitrary text

    Scenario: Heartbeat is sent at then
      Given Wait 10 ms ... some arbitrary text
      When Wait 10 ms ... some arbitrary text
      Then Some arbitrary text ... Node 0x1A is up

  Rule: PDO

    Scenario: PDO is sent at given
      Given Some arbitrary text ... PDO 0x1E5 payload 0x0102
      When Wait 10 ms ... some arbitrary text
      Then Wait 10 ms ... some arbitrary text

    Scenario: PDO is sent at when
      Given Wait 10 ms ... some arbitrary text
      When Some arbitrary text ... PDO 0x1E5 payload 0x0102
      Then Wait 10 ms ... some arbitrary text

    Scenario: PDO is expected
      Given Wait 10 ms ... some arbitrary text
      When Wait 10 ms ... some arbitrary text
      Then Some arbitrary text ... Expect PDO 0x1E5 payload 0x0102 within 500 ms

    Scenario: PDO is not expected
      Given Wait 10 ms ... some arbitrary text
      When Wait 10 ms ... some arbitrary text
      Then Some arbitrary text ... Reject PDO 0x1E5 payload 0x0102 within 500 ms

  Rule: SDO

    Scenario: SDO is sent at given
      Given Some arbitrary text ... Set object 0x8193,0x05 at node 0x11 as type u8 to value 0x01
      When Wait 10 ms ... some arbitrary text
      Then Wait 10 ms ... some arbitrary text

    Scenario: SDO is sent at when
      Given Wait 10 ms ... some arbitrary text
      When Some arbitrary text ... Set object 0x8193,0x05 at node 0x1A as type u8 to value 0x01
      Then Wait 10 ms ... some arbitrary text

    Scenario: SDO is expected at given
      Given Some arbitrary text ... Expect object 0x8193,0x05 at node 0x1A to be 0x01
      When Wait 10 ms ... some arbitrary text
      Then Wait 10 ms ... some arbitrary text

    Scenario: SDO is expected at then
      Given Wait 10 ms ... some arbitrary text
      When Wait 10 ms ... some arbitrary text
      Then Some arbitrary text ... Expect object 0x8193,0x05 at node 0x1A to be 0x01
