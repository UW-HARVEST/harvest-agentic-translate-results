// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
// 
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#include "driver.h"

#include <stdio.h>
#include <stdlib.h>

void printLine (const char * line)
{
    if(line != NULL) 
    {
        printf("%s\n", line);
    }
}

void printIntLine (int intNumber)
{
    printf("%d\n", intNumber);
}

void bad(int data)
{
    int i;
    int buffer[10] = { 0 };
    if (data >= 0)
    {
        buffer[data] = 1;
        /* Print the array values */
        for(i = 0; i < 10; i++)
        {
            printIntLine(buffer[i]);
        }
    }
    else
    {
        printLine("ERROR: Array index is negative.");
    }
}

static void goodG2B()
{
    int data = 7;
    int i;
    int buffer[10] = { 0 };
    if (data >= 0)
    {
        buffer[data] = 1;
        /* Print the array values */
        for(i = 0; i < 10; i++)
        {
            printIntLine(buffer[i]);
        }
    }
    else
    {
        printLine("ERROR: Array index is negative.");
    }
}

static void goodB2G(int data)
{
    int i;
    int buffer[10] = { 0 };
    if (data >= 0 && data < (10))
    {
        buffer[data] = 1;
        /* Print the array values */
        for(i = 0; i < 10; i++)
        {
            printIntLine(buffer[i]);
        }
    }
    else
    {
        printLine("ERROR: Array index is out-of-bounds");
    }
}

void good(int data)
{
    goodG2B();
    goodB2G(data);
}

void driver(int goodData, int badData)
{
    printLine("Calling good()...");
    good(goodData);
    printLine("Finished good()");
    printLine("Calling bad()...");
    bad(badData);
    printLine("Finished bad()");
}
