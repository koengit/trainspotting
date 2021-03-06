module Main where

import SAT as S
import SAT.Val
import SAT.Term
import SAT.Equal
import SAT.Order
import SAT.Bool
import Data.List( transpose )
import Thing

--------------------------------------------------------------------------------

-- this could be changed to another number representation, if that works better
type Color = Val Int

trivialColor :: Color
trivialColor = val 1

newColor :: Solver -> Int -> IO Color
newColor s n = newVal s [1..n]

isColorOr :: Solver -> [Lit] -> Color -> Int -> IO ()
isColorOr s xs numb k = addClause s ((numb .= k) : xs)

{-
-- Term works much worse
type Color = Term

trivialColor :: Color
trivialColor = fromList []

newColor :: Solver -> Int -> IO Color
newColor s n = newTerm s (fromIntegral (n-1))

isColorOr :: Solver -> [Lit] -> Color -> Int -> IO ()
isColorOr s xs numb k = equalOr s xs numb (number (fromIntegral (k-1)))
-}

--------------------------------------------------------------------------------

-- a cell maintains info on:
-- - what edges / \ _ are in it
-- - what color do they have
-- - the "point" in the lower-right corner
data Cell
  = Cell
  -- the edges in the cell
  { up      :: Lit -- /
  , down    :: Lit -- \
  , bot     :: Lit -- _

  , ccol    :: Color -- color of \ or /
  , bcol    :: Color -- color of _
  
  -- the point down-right
  , connect :: Lit -- is it connecting two edges?
  , thing   :: Lit -- is it one of the following features:
  , switchL :: Lit
  , switchR :: Lit
  , mergeL  :: Lit
  , mergeR  :: Lit
  , new     :: Lit
  , end     :: Lit
  }

newCell :: Solver -> Int -> IO Cell
newCell s nc =
  do x <- newLit s
     y <- newLit s
     z <- newLit s
     addClause s [neg x, neg y]
     c1 <- newColor s nc
     c2 <- newColor s nc
     
     cn <- newLit s
     th <- newLit s
     addClause s [neg cn, neg th]
     sL <- newLit s
     sR <- newLit s
     mL <- newLit s
     mR <- newLit s
     nw <- newLit s
     ed <- newLit s
     addClause s [neg th, sL, sR, mL, mR, nw, ed]
     return (Cell x y z c1 c2 cn th sL sR mL mR nw ed)

-- special cells at the border of the grid
newTopCell :: Solver -> Int -> IO Cell
newTopCell s nc =
  do z <- newLit s
     c2 <- newColor s nc
     
     cn <- newLit s
     th <- newLit s
     addClause s [neg cn, neg th]
     sR <- newLit s
     mL <- newLit s
     nw <- newLit s
     ed <- newLit s
     addClause s [neg th, sR, mL, nw, ed]
     return (Cell false false z trivialColor c2 cn th false sR mL false nw ed)

newLeftCell :: Solver -> IO Cell
newLeftCell s =
  do th <- newLit s
     nw <- newLit s
     addClause s [neg th, nw]
     return (Cell false false false trivialColor trivialColor
                  false th false false false false nw false)

-- used for bottom cells and right cells
emptyCell :: Cell
emptyCell =
  Cell false false false trivialColor trivialColor
       false false false false false false false false

--------------------------------------------------------------------------------

-- putting 4 cells next/above each other
cells4 :: Solver -> Int -> Cell -> Cell -> Cell -> Cell -> IO ()
cells4 s nc ul ur bl br =
  do -- exclude impossible configurations with too many edges
     addClause s [neg downL, neg upR]                       -- no \/
     addClause s [neg upL,   neg downR]                     -- no /\
     addClause s [neg downL, neg upL]                       -- no >
     addClause s [neg downR, neg upR]                       -- no <
     addClause s [neg downL, neg strtL, neg strtR, neg downR] -- no -\-
     addClause s [neg strtL, neg upL,   neg upR, neg strtR]   -- no -/-

     -- connect or thing when something is going on
     addClause s [neg downL, connect p, thing p]
     addClause s [neg strtL, connect p, thing p]
     addClause s [neg upL,   connect p, thing p]
     addClause s [neg downR, connect p, thing p]
     addClause s [neg strtR, connect p, thing p]
     addClause s [neg upR,   connect p, thing p]
     
     -- Connect
     addClause s [neg (connect p), downL, strtL, upL]
     addClause s [neg (connect p), neg downL, neg strtL]
     addClause s [neg (connect p), neg upL, neg strtL]
     addClause s [neg (connect p), downR, strtR, upR]
     addClause s [neg (connect p), neg downR, neg strtR]
     addClause s [neg (connect p), neg upR, neg strtR]

     -- connect makes sure that the edges have the same color
     col <- newColor s nc
     equalOr s [neg (connect p), neg downL] col cdownL
     equalOr s [neg (connect p), neg strtL] col cstrtL
     equalOr s [neg (connect p), neg upL]   col cupL
     equalOr s [neg (connect p), neg downR] col cdownR
     equalOr s [neg (connect p), neg strtR] col cstrtR
     equalOr s [neg (connect p), neg upR]   col cupR

     -- New
     addClause s [neg (new p), neg downL]
     addClause s [neg (new p), neg strtL]
     addClause s [neg (new p), neg upL]
     addClause s [neg (new p), neg upR]
     addClause s [neg (new p), strtR]
     addClause s [neg (new p), neg downR]

     -- End
     addClause s [neg (end p), neg downL]
     addClause s [neg (end p), strtL]
     addClause s [neg (end p), neg upL]
     addClause s [neg (end p), neg upR]
     addClause s [neg (end p), neg strtR]
     addClause s [neg (end p), neg downR]

     -- SwitchL
     addClause s [neg (switchL p), strtL, downL]
     addClause s [neg (switchL p), strtR]
     addClause s [neg (switchL p), neg strtL, upR]
     addClause s [neg (switchL p), neg downL, downR]

     -- SwitchR
     addClause s [neg (switchR p), strtL, upL]
     addClause s [neg (switchR p), strtR]
     addClause s [neg (switchR p), neg strtL, downR]
     addClause s [neg (switchR p), neg upL, upR]

     -- MergeL
     addClause s [neg (mergeL p), strtR, downR]
     addClause s [neg (mergeL p), strtL]
     addClause s [neg (mergeL p), neg strtR, upL]
     addClause s [neg (mergeL p), neg downR, downL]

     -- MergeR
     addClause s [neg (mergeR p), strtR, upR]
     addClause s [neg (mergeR p), strtL]
     addClause s [neg (mergeR p), neg strtR, downL]
     addClause s [neg (mergeR p), neg upR, upL]
 where
  p     = ul
  
  downL = down ul
  strtL = bot ul
  upL   = up bl
  upR   = up ur
  strtR = bot ur
  downR = down br
  
  cdownL = ccol ul
  cstrtL = bcol ul
  cupL   = ccol bl
  cupR   = ccol ur
  cstrtR = bcol ur
  cdownR = ccol br

--------------------------------------------------------------------------------

type Grid = [[Cell]]

newGrid :: Solver -> Int -> (Int,Int) -> IO Grid
newGrid s nc (x,y) =
  do css <- sequence
            [ sequence [ if j == y || i == x then
                           return emptyCell
                         else if i == 0 then
                           newLeftCell s
                         else if j == 0 then
                           newTopCell s nc
                         else
                           newCell s nc
                       | i <- [0..x]
                       ]
            | j <- [0..y]
            ]
     sequence_
       [ cells4 s nc ul ur bl br
       | (cs1,cs2) <- css `zip` tail css
       , let ccss = cs1 `zip` cs2
       , ((ul,bl),(ur,br)) <- ccss `zip` tail ccss
       ]
     return css

--------------------------------------------------------------------------------

-- adding constraints for a thing, adding the right colors
thingOr :: Solver -> [Lit] -> Thing -> Cell -> Cell -> Cell -> Cell -> IO ()
thingOr s pre th ul ur bl br =
  case th of
    New i ->
      do addClause s (new p : pre)
         isColorOr s pre cstrtR i

    End i ->
      do addClause s (end p : pre)
         isColorOr s pre cstrtL i

    SwitchL i j k ->
      do addClause s (switchL p : pre)
         isColorOr s (neg strtL : pre) cstrtL i
         isColorOr s (neg strtL : pre) cupR   j
         isColorOr s (neg strtL : pre) cstrtR k
         isColorOr s (neg downL : pre) cdownL i
         isColorOr s (neg downL : pre) cstrtR j
         isColorOr s (neg downL : pre) cdownR k

    SwitchR i j k ->
      do addClause s (switchR p : pre)
         isColorOr s (neg strtL : pre) cstrtL i
         isColorOr s (neg strtL : pre) cstrtR j
         isColorOr s (neg strtL : pre) cdownR k
         isColorOr s (neg upL   : pre) cupL   i
         isColorOr s (neg upL   : pre) cupR   j
         isColorOr s (neg upL   : pre) cstrtR k

    MergeL i j k ->
      do addClause s (mergeL p : pre)
         isColorOr s (neg strtR : pre) cstrtR k
         isColorOr s (neg strtR : pre) cstrtL i
         isColorOr s (neg strtR : pre) cupL   j
         isColorOr s (neg downR : pre) cdownR k
         isColorOr s (neg downR : pre) cdownL i
         isColorOr s (neg downR : pre) cstrtL j

    MergeR i j k ->
      do addClause s (mergeR p : pre)
         isColorOr s (neg strtR : pre) cstrtR k
         isColorOr s (neg strtR : pre) cdownL i
         isColorOr s (neg strtR : pre) cstrtL j
         isColorOr s (neg upR   : pre) cupR   k
         isColorOr s (neg upR   : pre) cstrtL i
         isColorOr s (neg upR   : pre) cupL   j
 where
  p     = ul
  
  downL = down ul
  strtL = bot ul
  upL   = up bl
  upR   = up ur
  strtR = bot ur
  downR = down br
  
  cdownL = ccol ul
  cstrtL = bcol ul
  cupL   = ccol bl
  cupR   = ccol ur
  cstrtR = bcol ur
  cdownR = ccol br

-- each point, if it is a thing, it must be a thing from the list
-- each thing from the list is represented exactly once
-- the things are in the right order
thingsGrid :: Solver -> [Thing] -> Grid -> IO ()
thingsGrid s ths css =
  do -- each point, if it's a thing, it needs to be in the list
     xlss <- sequence
              [ do ls <- sequence
                         [ do l <- newLit s
                              thingOr s [neg l] th ul ur bl br
                              return l
                         | th <- ths
                         ]
                   addClause s (neg (thing p) : ls)
                   return (x,ls)
              | (cs1,cs2) <- css `zip` tail css
              , let ccss = cs1 `zip` cs2
              , (x,((p@ul,bl),(ur,br))) <- [1..] `zip` (ccss `zip` tail ccss)
              ]
     -- each thing from the list exists exactly once
     sequence_
       [ do addClause s qs
            atMostOne s qs
       | qs <- transpose (map snd xlss)
       ]
     -- they appear in the same linear order
     sequence_
       [ addClause s (neg l : [ ls'!!(t-1) | (x',ls') <- xlss, x' <= x ])
       | (x,ls) <- xlss
       , (t,l)  <- [0..] `zip` ls
       , t > 0
       ]
     -- they appear in the same linear order, towards the right
     sequence_
       [ addClause s (neg l : [ ls'!!(t+1) | (x',ls') <- xlss, x' >= x ])
       | (x,ls) <- xlss
       , (t,l)  <- [0..] `zip` ls
       , t < length ls - 1
       ]

--------------------------------------------------------------------------------

main :: IO ()
main =
  do --layout example0
     --layout example1
     --layout example110
     layout exampleBjornar
     --layout exampleBjornar2
     --layout exampleBjornar2par
     --layout exampleBjornar3
     --layout exampleBjornar4

layout :: Example -> IO ()
layout example =
  withNewSolver $ \s ->
    do putStrLn "--- generating grid..."
       putStrLn ("(x,y) = " ++ show (x,y))
       putStrLn ("nc    = " ++ show nc)
       css <- newGrid s nc (x,y)
       putStrLn "--- generating things..."
       sequence_ [ print t | t <- things ]
       thingsGrid s things css
       putStrLn "--- solving..."
       b <- solve s []
       if b then
         do n <- displayGrid s css
            putStrLn "--- optimizing diags..."
            q <- newTerm s (fromIntegral n)
            ls <- sequence
                  [ do l <- newLit s
                       addClause s [l, neg (down c)]
                       addClause s [l, neg (up c)]
                       return l
                  | cs <- css
                  , c <- cs
                  ]
            
            lessThanEqual s (fromList [(1,l) | l <- ls]) q
            a <- newLit s
            let loop a n =
                  do lessThanEqualOr s [neg a] q (number (fromIntegral n))
                     b <- solve s [a]
                     if b then
                       do n' <- displayGrid s css
                          loop a (n'-1)
                      else
                       do putStrLn "NO BETTER SOLUTIONS"
             in loop a (n-1)
        else
         do putStrLn "NO"
 where
  ((x,y),things0) = example
  things          = rename things0
  nc              = maximum [ c | th <- things, c <- colors th ]

displayGrid :: Solver -> Grid -> IO Int
displayGrid s css =
  do bs <- sequence
                  [ do b1 <- S.modelValue s (down c)
                       b2 <- S.modelValue s (up c)
                       return (b1 || b2)
                  | cs <- css
                  , c <- cs
                  ]
     let n = length (filter id bs)
     putStrLn ("diags = " ++ show n ++ ":")
     sequence_
       [ do sequence_
              [ do a <- S.modelValue s (down c)
                   putStr (if a then "\\" else " ")
                   b <- S.modelValue s (up c)
                   putStr (if b then "/" else " ")
              | c <- cs
              ]
            putStrLn ""
            sequence_
              [ do d <- S.modelValue s (bot c)
                   b <- S.modelValue s (up c)
                   putStr (if b then "/" else if d then "_" else " ")
                   a <- S.modelValue s (down c)
                   putStr (if a then "\\" else if d then "_" else ".")

                   --th <- S.modelValue s (thing c)
                   --cn <- S.modelValue s (connect c)
                   --putStr (if th then "T" else if cn then "C" else " ")
              | c <- cs
              ]
            putStrLn ""
       | cs <- (init . tail) `fmap` init css
       ]
     return n


